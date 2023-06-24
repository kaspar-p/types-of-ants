import { SiteData, getSiteData, getRawData, RawData } from "./parse";
import { execSync } from "child_process";
import fs from "fs";
import {
  antReleaseSql,
  antTweetedSql,
  antsToSql,
  declinedToSql,
  releaseSql,
} from "./sql";

export type UnseenAnt = {
  originalSuggestionContent: string;
  createdAt: string;
};
export type DeclinedAnt = {
  closedAt: string;
  originalSuggestionContent: string;
  createdAt: string;
};
export type AcceptedAntWithRelease = AcceptedAnt & {
  release: number;
};
export type AcceptedAnt = {
  ordering: number;
  closedAt: string;
  antContent: string;
  tweeted: boolean;
  tweetedAt: string | null;
  originalSuggestionContent: string;
  createdAt: string;
};
export type LegacyAntWithRelease = LegacyAnt & {
  release: number;
};
export type LegacyAnt = {
  ordering: number;
  antContent: string;
  createdAt: string;
  tweeted: boolean;
  tweetedAt: string | null;
  originalSuggestionContent: string;
};

export type AntMetadata =
  | LegacyAntWithRelease
  | AcceptedAntWithRelease
  | DeclinedAnt
  | UnseenAnt;

export type SiteAntMetadata = {
  antContent: string;
  originalSuggestionContent: string;
  ordering: number;
  tweeted: boolean;
  tweetedAt: string | null;
};

// Content is either 'ant' or 'suggestedContent'
const siteData = getSiteData();
const forwardMap = new Map<string, SiteAntMetadata>();
const backwardMap = new Map<string, SiteAntMetadata>();
siteData.ants.forEach((ant) => {
  forwardMap.set(ant.ant, createMetadataFromSiteAnt(ant));
  if (ant.suggestedContent) {
    backwardMap.set(ant.suggestedContent, createMetadataFromSiteAnt(ant));
  }
});

function getSiteAntFromContent(content: string): SiteAntMetadata | undefined {
  const fromForward = forwardMap.get(content);
  if (fromForward) return fromForward;
  const fromBackward = backwardMap.get(content);
  if (fromBackward) {
    return fromBackward;
  }
  // throw new Error("Content is not linked to ANY ant: " + content);
  return undefined;
}

function hashCode(str: string): number {
  let hash = 0;
  for (let i = 0, len = str.length; i < len; i++) {
    let chr = str.charCodeAt(i);
    hash = (hash << 5) - hash + chr;
    hash |= 0; // Convert to 32bit integer
  }
  return hash >>> 0;
}

function createMetadataFromSiteAnt(
  antFromSite: SiteData["ants"][number]
): SiteAntMetadata {
  return {
    antContent: antFromSite.ant,
    originalSuggestionContent: antFromSite.suggestedContent ?? antFromSite.ant,
    ordering: hashCode(antFromSite.ant),
    tweeted: antFromSite.tweeted ?? false,
    tweetedAt: antFromSite.tweetedAt ?? null,
  };
}

function parseTitleForAntContent(title: string): string | undefined {
  const titleRegex = /`(.+)`/g;
  const m = title.match(titleRegex);
  if (!m) {
    // console.log("Title didn't match regex: " + title);
    return;
  }
  const antContent = m[0].slice(1, m[0].length - 1);
  return antContent;
}

function getAllUnseenAnts(): UnseenAnt[] {
  const rawData = getRawData();

  const unseenAnts: UnseenAnt[] = [];
  rawData.forEach((issue) => {
    if (issue.closed === false) {
      const antContent = parseTitleForAntContent(issue.title);
      if (!antContent) return;
      unseenAnts.push({
        originalSuggestionContent: antContent,
        createdAt: issue.createdAt,
      });
    }
  });

  return unseenAnts;
}

const getCommitsAntWasPresent = (antContent: string): string => {
  const safeAntContent = [...antContent]
    .map((c) => c.replace("'", "\\'"))
    .join("");
  return `git log --oneline -S $'${safeAntContent}' | tail -1`;
};
const getCommitIdFromOneline = (inputLine: string): string => {
  return `echo '${inputLine}' | awk '{print $1}'`;
};
// Using the 'git log' history, get the earliest mention
const formatSingleCommit = (gitCommitId: string) => {
  return `git log -n 1 '${gitCommitId}' --format=%cd`;
};

function getEarliestCommitDate(antContent: string): Date {
  if (!antContent) throw new Error("Passed nothing!");
  const lastCommitLine: string = execSync(getCommitsAntWasPresent(antContent))
    .toString("utf-8")
    .trim();
  if (!lastCommitLine)
    throw new Error("Content was never seen?: " + antContent);
  const commitId: string = execSync(getCommitIdFromOneline(lastCommitLine))
    .toString("utf-8")
    .trim();
  if (!commitId)
    throw new Error("The parsing line didn't work?: " + antContent);
  const singleCommitDate: string = execSync(formatSingleCommit(commitId))
    .toString("utf-8")
    .trim();
  return new Date(singleCommitDate);
}

type WithReleases = {
  acceptedAnts: AcceptedAntWithRelease[];
  legacyAnts: LegacyAntWithRelease[];
};
function assignReleasesToAnts(
  acceptedAnts: AcceptedAnt[],
  legacyAnts: LegacyAnt[]
): WithReleases {
  const uniqueCommitDays: Record<string, (AcceptedAnt | LegacyAnt)[]> = {};
  [...acceptedAnts, ...legacyAnts].forEach((ant: AcceptedAnt | LegacyAnt) => {
    const dateString = new Date(ant.createdAt).toDateString();
    if (!(dateString in uniqueCommitDays)) uniqueCommitDays[dateString] = [];
    uniqueCommitDays[dateString].push(ant);
  });

  const dateToReleaseNumber: Record<string, number> = {};
  Object.keys(uniqueCommitDays)
    .sort((a, b) => {
      return new Date(a).getTime() - new Date(b).getTime();
    })
    .forEach((dateString, i) => {
      // Start the releases at 1
      dateToReleaseNumber[dateString] = i + 1;
    });

  return {
    acceptedAnts: acceptedAnts.map((ant) => ({
      ...ant,
      release: dateToReleaseNumber[new Date(ant.createdAt).toDateString()],
    })),
    legacyAnts: legacyAnts.map((ant) => ({
      ...ant,
      release: dateToReleaseNumber[new Date(ant.createdAt).toDateString()],
    })),
  };
}

function getAllLegacyAnts(): LegacyAnt[] {
  const siteData = getSiteData();
  // Get the ants marked as legacy
  const legacy: LegacyAnt[] = siteData.ants
    .filter((ant) => ant.legacy)
    .map((ant) => {
      return {
        antContent: ant.ant,
        tweeted: ant.tweeted ?? false,
        tweetedAt: ant.tweetedAt ?? null,
        originalSuggestionContent: ant.ant,
        ordering: hashCode(ant.ant),
        createdAt: getEarliestCommitDate(ant.ant).toISOString(),
      };
    });

  return dedupe(legacy, "antContent");
}

function getAllDeclinedAndAcceptedAnts(): {
  declinedAnts: DeclinedAnt[];
  acceptedAnts: AcceptedAnt[];
} {
  const rawData = getRawData();

  const declinedAnts: DeclinedAnt[] = [];
  const acceptedAnts: AcceptedAnt[] = [];

  rawData.forEach((issue) => {
    if (issue.closed === false || issue.closedAt === null) return;

    const content = parseTitleForAntContent(issue.title);
    if (!content) return;

    const siteAnt = getSiteAntFromContent(content);
    if (!siteAnt) {
      // console.log(`Ant '${content}' was declined!`);
      declinedAnts.push({
        originalSuggestionContent: content,
        createdAt: issue.createdAt,
        closedAt: issue.closedAt,
      });
    } else {
      // console.log(`Ant '${siteAnt.antContent}' was on the site!`);
      acceptedAnts.push({
        antContent: siteAnt.antContent,
        ordering: siteAnt.ordering,
        tweeted: siteAnt.tweeted,
        tweetedAt: siteAnt.tweetedAt,
        originalSuggestionContent: siteAnt.originalSuggestionContent,
        createdAt: issue.createdAt,
        closedAt: issue.closedAt,
      });
    }
  });

  return {
    declinedAnts: declinedAnts,
    acceptedAnts: dedupe(acceptedAnts, "antContent"),
  };
}

function dedupe<T, K extends string>(arr: Record<K, unknown>[], key: K): T[] {
  const m = new Map();
  for (const x of arr) {
    m.set(x[key], x);
  }

  return Array.from(m.values());
}

function formatAnts(ants: AntMetadata[]): string {
  return ants
    .map((ant) =>
      "antContent" in ant
        ? " -> " + ant.antContent
        : " -> " + ant.originalSuggestionContent
    )
    .join("\n");
}

function formatAntsWithReleases(
  ants: (LegacyAntWithRelease | AcceptedAntWithRelease)[]
): string {
  return ants
    .map((ant) => " -> " + ant.release + " ::: " + ant.antContent)
    .join("\n");
}

function main() {
  const { acceptedAnts: acceptedAntsWithoutReleases, declinedAnts } =
    getAllDeclinedAndAcceptedAnts();

  // console.log("LEGACY ANTS: \n", formatAnts(legacyAnts), legacyAnts.length);
  // console.log("ACCEPTED ANTS: \n", formatAnts(acceptedAnts), acceptedAnts.length);
  // console.log("DECLINED ANTS: \n", formatAnts(declinedAnts), declinedAnts.length);

  const { acceptedAnts, legacyAnts } = assignReleasesToAnts(
    acceptedAntsWithoutReleases,
    getAllLegacyAnts()
  );

  const allAnts: AntMetadata[] = [
    ...acceptedAnts,
    ...legacyAnts,
    ...declinedAnts,
  ];
  const siteAnts: (LegacyAntWithRelease | AcceptedAntWithRelease)[] = [
    ...acceptedAnts,
    ...legacyAnts,
  ];
  const sqlFor_ant = antsToSql(allAnts);
  fs.writeFileSync("./sql_output/ant.sql", sqlFor_ant, { encoding: "utf-8" });

  const sqlFor_ant_tweeted = antTweetedSql(siteAnts);
  fs.writeFileSync("./sql_output/ant_tweeted.sql", sqlFor_ant_tweeted, {
    encoding: "utf-8",
  });

  const sqlFor_ant_declined = declinedToSql(declinedAnts);
  fs.writeFileSync("./sql_output/ant_declined.sql", sqlFor_ant_declined, {
    encoding: "utf-8",
  });

  const sqlFor_ant_release = antReleaseSql(siteAnts);
  fs.writeFileSync("./sql_output/ant_release.sql", sqlFor_ant_release, {
    encoding: "utf-8",
  });

  const sqlFor_release = releaseSql(siteAnts);
  fs.writeFileSync("./sql_output/release.sql", sqlFor_release, {
    encoding: "utf-8",
  });
}

main();
