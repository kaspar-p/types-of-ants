use std::format;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};

/// A window of calendar time in UTC. Fixed-length periods are aligned to the Unix epoch, so
/// weeks start on a Thursday. Months are calendar months, since they vary in length.
#[derive(Clone, Copy)]
enum Period {
    Fixed(Duration),
    Month,
}

impl Period {
    /// Numbers periods consecutively, so a number always means the same window of time.
    fn index(&self, time: DateTime<Utc>) -> i64 {
        match self {
            Period::Fixed(length) => time.timestamp().div_euclid(length.num_seconds()),
            Period::Month => i64::from(time.year()) * 12 + i64::from(time.month0()),
        }
    }

    fn start(&self, index: i64) -> DateTime<Utc> {
        match self {
            Period::Fixed(length) => DateTime::from_timestamp(index * length.num_seconds(), 0)
                .expect("period starts before the backup in it"),
            Period::Month => Utc
                .with_ymd_and_hms(
                    index.div_euclid(12) as i32,
                    index.rem_euclid(12) as u32 + 1,
                    1,
                    0,
                    0,
                    0,
                )
                .single()
                .expect("period starts before the backup in it"),
        }
    }

    fn name(&self) -> String {
        match self {
            Period::Fixed(length) => format!("{}h", length.num_hours()),
            Period::Month => "1mo".to_string(),
        }
    }
}

/// Keep the newest backup in each of the `count` most recent periods.
///
/// Periods are fixed windows of time, not ages relative to now, so the period a backup belongs
/// to never changes as time passes. That is what makes deleting safe: a backup that isn't the
/// newest in a recent period can never become one later. Only periods that contain a backup
/// count towards `count`, so if backups stop being taken the existing ones are never rotated
/// away.
struct Retention {
    period: Period,
    count: usize,
}

fn build_retention() -> Vec<Retention> {
    vec![
        Retention {
            period: Period::Fixed(Duration::hours(1)),
            count: 6,
        },
        Retention {
            period: Period::Fixed(Duration::hours(6)),
            count: 4,
        },
        Retention {
            period: Period::Fixed(Duration::days(1)),
            count: 7,
        },
        Retention {
            period: Period::Fixed(Duration::weeks(1)),
            count: 5,
        },
        Retention {
            period: Period::Month,
            count: 6,
        },
    ]
}

pub struct Slot {
    period: Period,
    start: DateTime<Utc>,
}

#[derive(Clone)]
pub struct SlotEntry {
    pub id: String,
    pub key: String,
    pub created: chrono::DateTime<Utc>,
}

impl PartialEq for SlotEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl ToString for SlotEntry {
    fn to_string(&self) -> String {
        format!("{} ({} @ {})", self.id, self.key, self.created.to_rfc2822())
    }
}

pub struct Slots<'a> {
    pub slots: Vec<(Slot, &'a SlotEntry)>,
    pub unused_backups: Vec<&'a SlotEntry>,
}

impl<'a> Slots<'a> {
    pub fn new(backups: &'a [SlotEntry]) -> Self {
        // Newest first. Ties are broken by id so the result doesn't depend on input order.
        let mut sorted_backups: Vec<&'a SlotEntry> = backups.iter().collect();
        sorted_backups.sort_by(|a, b| (b.created, &b.id).cmp(&(a.created, &a.id)));

        let mut slots: Vec<(Slot, &'a SlotEntry)> = vec![];

        for retention in build_retention() {
            let mut filled = 0;
            let mut last_period = None;

            for backup in &sorted_backups {
                if filled == retention.count {
                    break;
                }

                // Backups are newest first, so the first one seen in a period is its newest.
                let period = retention.period.index(backup.created);
                if last_period == Some(period) {
                    continue;
                }
                last_period = Some(period);
                filled += 1;

                slots.push((
                    Slot {
                        period: retention.period,
                        start: retention.period.start(period),
                    },
                    backup,
                ));
            }
        }

        let unused_backups = sorted_backups
            .into_iter()
            .filter(|b| !slots.iter().any(|(_, kept)| kept == b))
            .collect();

        Slots {
            slots,
            unused_backups,
        }
    }

    pub fn print(&self) -> String {
        self.slots
            .iter()
            .enumerate()
            .map(|(idx, (slot, backup))| {
                format!(
                    "[{}] {} from {}: {}",
                    idx,
                    slot.period.name(),
                    slot.start.to_rfc3339(),
                    backup.to_string(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod test {
    use crate::slots::{SlotEntry, Slots};
    use chrono::{DateTime, Duration, TimeZone, Utc};
    use std::collections::HashSet;

    use pretty_assertions::assert_eq;

    fn entry(id: &str, created: DateTime<Utc>) -> SlotEntry {
        SlotEntry {
            id: id.to_string(),
            key: id.to_string(),
            created,
        }
    }

    /// The upper bound on retained backups: one per hourly (6), 6-hourly (4), daily (7),
    /// weekly (5) and monthly (6) period.
    const MAX_RETAINED: usize = 6 + 4 + 7 + 5 + 6;

    /// A Thursday, the same weekday as the Unix epoch, so weekly periods start on it.
    fn t0() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn at(minutes: i64) -> DateTime<Utc> {
        t0() + Duration::minutes(minutes)
    }

    fn day(year: i32, month: u32, day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, 0, 0, 0).unwrap()
    }

    fn surviving(backups: &[SlotEntry]) -> Vec<SlotEntry> {
        let slots = Slots::new(backups);
        let unused: HashSet<&str> = slots.unused_backups.iter().map(|b| b.id.as_str()).collect();
        backups
            .iter()
            .filter(|b| !unused.contains(b.id.as_str()))
            .cloned()
            .collect()
    }

    fn ids(backups: &[SlotEntry]) -> Vec<String> {
        let mut ids: Vec<String> = backups.iter().map(|b| b.id.clone()).collect();
        ids.sort();
        ids
    }

    fn ages_in_hours(backups: &[SlotEntry], now: DateTime<Utc>) -> Vec<i64> {
        let mut ages: Vec<i64> = backups
            .iter()
            .map(|b| (now - b.created).num_hours())
            .collect();
        ages.sort();
        ages
    }

    /// Takes a backup at each of `times` and rotates after each one, deleting unused backups
    /// like `post_backup` does. Deletion can't be undone, so every run must keep exactly what
    /// a run over every backup ever taken would keep: it must never delete a backup that a
    /// later run would have wanted.
    fn march(times: &[DateTime<Utc>]) -> Vec<SlotEntry> {
        let mut kept: Vec<SlotEntry> = vec![];
        let mut taken: Vec<SlotEntry> = vec![];

        for (i, created) in times.iter().enumerate() {
            let backup = entry(&format!("backup_{i:05}"), *created);
            kept.push(backup.clone());
            taken.push(backup.clone());

            kept = surviving(&kept);

            assert_eq!(
                ids(&kept),
                ids(&surviving(&taken)),
                "run {i} @ {} deleted a backup it later needed",
                created.to_rfc3339()
            );
            assert!(
                kept.contains(&backup),
                "run {i} deleted the backup it just took"
            );
            assert!(
                kept.len() <= MAX_RETAINED,
                "run {i} kept {} backups",
                kept.len()
            );
        }

        kept
    }

    #[test]
    fn no_backups_keeps_nothing() {
        let slots = Slots::new(&[]);

        assert!(slots.unused_backups.is_empty());
    }

    #[test]
    fn single_backup_is_kept() {
        let entries = vec![entry("only", t0())];

        assert_eq!(ids(&surviving(&entries)), vec!["only"]);
    }

    #[test]
    fn hourly_backups_keep_one_per_period_after_90_days() {
        let times = (0..24 * 90).map(|h| at(h * 60)).collect::<Vec<_>>();

        let kept = march(&times);

        assert_eq!(
            ages_in_hours(&kept, *times.last().unwrap()),
            vec![
                0, 1, 2, 3, 4, 5, // hourly
                6, 12, 18, 24, // 6-hourly
                48, 72, 96, 120, 144, // daily
                312, 480, 648, // weekly
                744, 1416, // monthly: the ends of February and January
            ]
        );
    }

    #[test]
    fn daily_backups_keep_one_per_period_after_90_days() {
        let times = (0..90).map(|d| at(d * 24 * 60)).collect::<Vec<_>>();

        let kept = march(&times);

        assert_eq!(
            ages_in_hours(&kept, *times.last().unwrap()),
            vec![0, 24, 48, 72, 96, 120, 144, 312, 480, 648, 744, 1416]
        );
    }

    #[test]
    fn daily_backups_keep_the_end_of_each_month_for_six_months() {
        let times = (0..365).map(|d| at(d * 24 * 60)).collect::<Vec<_>>();

        let kept = march(&times);

        let older_than_december = kept
            .iter()
            .map(|b| b.created)
            .filter(|created| *created < day(2026, 12, 1))
            .collect::<Vec<_>>();
        assert_eq!(
            older_than_december,
            vec![
                day(2026, 7, 31),
                day(2026, 8, 31),
                day(2026, 9, 30),
                day(2026, 10, 31),
                day(2026, 11, 30),
            ]
        );
    }

    #[test]
    fn leap_day_is_the_end_of_february() {
        let times = (0..90)
            .map(|d| day(2028, 1, 1) + Duration::days(d))
            .collect::<Vec<_>>();

        let kept = march(&times);

        assert!(kept.iter().any(|b| b.created == day(2028, 2, 29)));
        assert!(!kept.iter().any(|b| b.created == day(2028, 2, 28)));
    }

    #[test]
    fn jittered_hourly_backups_are_never_deleted_early() {
        // Up to 20 minutes either side of the hour, so runs straddle period boundaries.
        let times = (0..24 * 45)
            .map(|h| at(h * 60 + (h * 7919) % 41 - 20))
            .collect::<Vec<_>>();

        march(&times);
    }

    #[test]
    fn frequent_backups_are_never_deleted_early() {
        let times = (0..6 * 24 * 30).map(|i| at(i * 10)).collect::<Vec<_>>();

        let kept = march(&times);

        assert_eq!(
            ages_in_hours(&kept, *times.last().unwrap()),
            vec![0, 1, 2, 3, 4, 5, 6, 12, 18, 24, 48, 72, 96, 120, 144, 216, 384, 552]
        );
    }

    #[test]
    fn irregular_backups_are_never_deleted_early() {
        // Gaps from 1 minute to ~2 days, from a fixed linear congruential sequence.
        let mut minutes = 0;
        let mut seed: i64 = 42;
        let times = (0..1000)
            .map(|_| {
                seed = (seed * 1103515245 + 12345) % 2147483648;
                minutes += 1 + seed % (2 * 24 * 60);
                at(minutes)
            })
            .collect::<Vec<_>>();

        march(&times);
    }

    #[test]
    fn outage_keeps_backups_from_before_the_outage() {
        let mut times = (0..24 * 40).map(|h| at(h * 60)).collect::<Vec<_>>();
        times.push(at((24 * 61) * 60));

        let kept = march(&times);

        // The first backup after 3 weeks of nothing evicts only the oldest backup of each
        // period, so the most recent hours before the outage are still around.
        let before_outage = kept
            .iter()
            .filter(|b| b.created >= at((24 * 40 - 5) * 60) && b.created < at(24 * 40 * 60))
            .count();
        assert_eq!(before_outage, 5);
    }

    #[test]
    fn rotating_without_new_backups_deletes_nothing() {
        let times = (0..24 * 10).map(|h| at(h * 60)).collect::<Vec<_>>();
        let kept = march(&times);

        let slots = Slots::new(&kept);

        assert!(
            slots.unused_backups.is_empty(),
            "deleted {:?}",
            slots
                .unused_backups
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn newest_backup_in_an_hour_replaces_earlier_ones() {
        let entries = vec![
            entry("middle", at(10)),
            entry("newest", at(20)),
            entry("earliest", at(0)),
        ];

        assert_eq!(ids(&surviving(&entries)), vec!["newest"]);
    }

    #[test]
    fn backup_on_the_hour_starts_a_new_period() {
        let entries = vec![
            entry("start_of_hour_0", at(1)),
            entry("end_of_hour_0", at(59)),
            entry("start_of_hour_1", at(60)),
            entry("later_in_hour_1", at(61)),
        ];

        assert_eq!(
            ids(&surviving(&entries)),
            vec!["end_of_hour_0", "later_in_hour_1"]
        );
    }

    #[test]
    fn rotation_does_not_depend_on_input_order() {
        let mut entries = (0..24 * 10)
            .map(|h| entry(&format!("backup_{h:05}"), at(h * 60)))
            .collect::<Vec<_>>();
        // Duplicate timestamps must still resolve the same way.
        entries.push(entry("duplicate_a", at(24 * 10 * 60)));
        entries.push(entry("duplicate_b", at(24 * 10 * 60)));

        let forwards = ids(&surviving(&entries));
        entries.reverse();
        let backwards = ids(&surviving(&entries));

        assert_eq!(forwards, backwards);
    }

    #[test]
    fn future_dated_backup_does_not_evict_new_backups() {
        // A backup from a host with a clock a year fast.
        let mut times = vec![at(365 * 24 * 60)];
        times.extend((0..24 * 10).map(|h| at(h * 60)));

        let kept = march(&times);

        // It costs one hourly period, but the rest still go to the newest real backups.
        let ages = ages_in_hours(&kept, at((24 * 10 - 1) * 60));
        assert!(ages[0] < 0);
        assert_eq!(ages[1..6], [0, 1, 2, 3, 4]);
    }
}
