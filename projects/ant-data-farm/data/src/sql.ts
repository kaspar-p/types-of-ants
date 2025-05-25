import {
  type DeclinedAnt,
  type AntMetadata,
  type LegacyAntWithRelease,
  type AcceptedAntWithRelease,
} from "./main";

export function hashCode(str: string): number {
  let hash = 0;
  for (let i = 0, len = str.length; i < len; i++) {
    let chr = str.charCodeAt(i);
    hash = (hash << 5) - hash + chr;
    hash |= 0; // Convert to 32bit integer
  }
  return (hash >>> 0) % (Math.pow(2, 31) - 1);
}

function sanitizeForSql(content: string): string {
  return content.replace(/'/g, "''");
}

function toSqlDate(date: string): string {
  return new Date(date).toISOString().slice(0, 19).replace("T", " ");
}

function antId(originalSuggestionContent: string, createdAt: string) {
  const sanitized = sanitizeForSql(originalSuggestionContent);
  return (
    `(select ant_id from ant where ` +
    `suggested_content = '${sanitized}'` +
    " and " +
    `created_at = '${toSqlDate(createdAt)}')`
  );
}

const userId = (user: "kaspar" | "nobody") =>
  `(select user_id from registered_user where user_name = '${user}')`;

export function antsToSql(
  ants: { createdAt: string; originalSuggestionContent: string }[]
): string {
  function singleRow(ant: AntMetadata): string {
    const { createdAt, originalSuggestionContent } = ant;
    const content = sanitizeForSql(originalSuggestionContent);
    const date = toSqlDate(createdAt);
    return `('${content}', ${userId("nobody")}, '${date}')`;
  }

  const rows = ants
    .map(singleRow)
    .map((row) => "    " + row)
    .join(",\n");

  return `insert into ant (suggested_content, ant_user_id, created_at)
  values
${rows}
;`;
}

export function antTweetedSql(
  ants: (LegacyAntWithRelease | AcceptedAntWithRelease)[]
): string {
  function singleRow(
    ant: LegacyAntWithRelease | AcceptedAntWithRelease
  ): string {
    if (ant.tweetedAt === null) {
      throw new Error("tweetedAt was null for: " + ant.antContent);
    }
    const timestamp = toSqlDate(ant.tweetedAt);
    const ant_id = antId(ant.originalSuggestionContent, ant.createdAt);
    return `(${ant_id}, '${timestamp}')`;
  }

  const rows = ants
    .filter((ant) => ant.tweeted === true && ant.tweetedAt !== null)
    .map(singleRow)
    .map((row) => "    " + row)
    .join(",\n");

  return `insert into ant_tweeted (ant_id, tweeted_at)
  values
${rows}
;`;
}

export function declinedToSql(ants: DeclinedAnt[]): string {
  function singleRow(ant: DeclinedAnt): string {
    const ant_id = antId(ant.originalSuggestionContent, ant.createdAt);
    const user_id = userId("kaspar");
    return `(${ant_id}, ${user_id}, '${toSqlDate(ant.closedAt)}')`;
  }

  const rows = ants
    .map(singleRow)
    .map((row) => "    " + row)
    .join(",\n");

  return `insert into ant_declined (ant_id, ant_declined_user_id, ant_declined_at)
  values
${rows}
;`;
}

export function releaseSql(releases: number[]): string {
  function singleRow(release: number): string {
    const label = `v${release}`;
    return `(${release}, '${label}')`;
  }

  const uniqueReleases = new Set<number>(releases);

  const rows = Array.from(uniqueReleases)
    .sort((a, b) => a - b)
    .map(singleRow)
    .map((row) => "    " + row)
    .join(",\n");

  return `insert into release (release_number, release_label)
  values
${rows}
;`;
}

export function antReleaseSql(
  ants: {
    originalSuggestionContent: string;
    createdAt: string;
    antContent: string;
    release: number;
    ordering: number;
  }[]
): string {
  function singleRow(ant: {
    originalSuggestionContent: string;
    createdAt: string;
    antContent: string;
    release: number;
    ordering: number;
  }): string {
    const ant_id = antId(ant.originalSuggestionContent, ant.createdAt);
    const content = sanitizeForSql(ant.antContent);
    return `(${ant_id}, ${ant.release}, '${content}', ${ant.ordering})`;
  }

  const rows = ants
    .map(singleRow)
    .map((row) => "    " + row)
    .join(",\n");

  return `insert into ant_release (ant_id, release_number, ant_content, ant_content_hash)
  values
${rows}
;`;
}

export function migrationSql(migrationLabel: string) {
  const value = `    ('${sanitizeForSql(migrationLabel)}', now(), now())`;

  return `insert into migration (migration_label, created_at, updated_at)
  values
${value}
;`;
}

// function legacyAntsToSql(ants: LegacyAnt[]): string {
//   //
// }

// function unseenAntsToSql(ants: UnseenAnt[]): string {
//   //
// }

// function declinedAntsToSql(ants: DeclinedAnt[]): string {
//   //
// }

// function acceptedAntsToSql(ants: AcceptedAnt[]): string {
//   //
// }
