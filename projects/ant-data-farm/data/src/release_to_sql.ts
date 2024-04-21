import * as fs from "fs-extra";
import { antReleaseSql, antsToSql, releaseSql } from "./sql";

type ReleasesFile = {
  Date: {
    Year: number;
    Month: number;
    Day: number;
  };
  Ants: string[];
};

function makeDate(date: { Year: number; Month: number; Day: number }): Date {
  const d = new Date();
  d.setFullYear(date.Year);
  d.setMonth(date.Month);
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

  if (!fs.existsSync(releasesFile)) {
    throw new Error(releasesFile + " does not exist!");
  }

  const content = fs.readFileSync(releasesFile, { encoding: "utf-8" });
  const release: ReleasesFile = JSON.parse(content);

  const tableChange_ant = antsToSql(
    release.Ants.map((ant) => ({
      createdAt: makeDate(release.Date).toISOString(),
      originalSuggestionContent: ant,
    }))
  );

  const tableChange_ant_release = antReleaseSql(
    release.Ants.map((ant, i) => ({
      antContent: ant,
      originalSuggestionContent: ant,
      createdAt: makeDate(release.Date).toISOString(),
      release: releaseNumber,
      ordering: i,
    }))
  );

  const tableChange_release = releaseSql([releaseNumber]);

  console.log(tableChange_ant);
  console.log();
  console.log(tableChange_release);
  console.log();
  console.log(tableChange_ant_release);
}

main();
