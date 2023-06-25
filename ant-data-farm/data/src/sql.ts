import {
  LegacyAnt,
  AcceptedAnt,
  UnseenAnt,
  DeclinedAnt,
  AntMetadata,
  LegacyAntWithRelease,
  AcceptedAntWithRelease,
} from "./main";

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

export function antsToSql(ants: AntMetadata[]): string {
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

export function releaseSql(
  ants: (LegacyAntWithRelease | AcceptedAntWithRelease)[]
): string {
  function singleRow(release: number): string {
    const label = `v${release}`;
    return `(${release}, '${label}')`;
  }

  const uniqueReleases = new Set<number>();
  ants.forEach((ant) => uniqueReleases.add(ant.release));

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
  ants: (LegacyAntWithRelease | AcceptedAntWithRelease)[]
): string {
  function singleRow(
    ant: LegacyAntWithRelease | AcceptedAntWithRelease
  ): string {
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
