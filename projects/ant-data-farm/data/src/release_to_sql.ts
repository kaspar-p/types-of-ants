import { readFileSync, existsSync } from "fs-extra";
import {
  hashCode,
  antReleaseSql,
  antsToSql,
  releaseSql,
  migrationSql,
} from "./sql.js";
import assert from "assert";

type ReleasesFile = {
  Date: {
    Year: number;
    Month: number;
    Day: number;
  };
  Ants: string[];
};

function makeDate(date: { Year: number; Month: number; Day: number }): Date {
  // JavaScript Date has 0 as January, but our data saves 1-12.
  const zeroIndexedMonth = date.Month - 1;
  assert(zeroIndexedMonth >= 0);
  assert(zeroIndexedMonth <= 11);

  const d = new Date();
  d.setFullYear(date.Year);
  d.setMonth(zeroIndexedMonth);
  d.setDate(date.Day);
  d.setHours(0);
  d.setMinutes(0);
  d.setSeconds(0);
  return d;
}

function main() {
  if (process.argv.length !== 4) {
    throw new Error("Needs <releaseFile> <releaseNumber> arguments!");
  }

  const releasesFile: string = process.argv[2];
  const releaseNumber: number = parseInt(process.argv[3]);

  if (!existsSync(releasesFile)) {
    throw new Error(releasesFile + " does not exist!");
  }

  const content = readFileSync(releasesFile, { encoding: "utf-8" });
  const release: ReleasesFile = JSON.parse(content);

  const tableChange_ant = antsToSql(
    release.Ants.map((ant) => ({
      createdAt: makeDate(release.Date).toISOString(),
      originalSuggestionContent: ant,
    }))
  );

  const tableChange_ant_release = antReleaseSql(
    release.Ants.map((ant) => ({
      antContent: ant,
      originalSuggestionContent: ant,
      createdAt: makeDate(release.Date).toISOString(),
      release: releaseNumber,
      ordering: hashCode(ant),
    }))
  );

  const tableChange_release = releaseSql([releaseNumber]);

  const tableChange_migration = migrationSql(
    `ant-release:${release.Date.Year}.${release.Date.Month}.${release.Date.Day}`
  );

  console.log("BEGIN;");
  console.log();
  console.log(tableChange_ant);
  console.log();
  console.log(tableChange_release);
  console.log();
  console.log(tableChange_ant_release);
  console.log();
  console.log(tableChange_migration);
  console.log();
  console.log("COMMIT;");
}

main();
