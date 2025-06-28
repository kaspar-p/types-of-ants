import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { hashCode, sanitizeForSql, migrationSql } from "./sql.js";

function main() {
  const root = execSync("git rev-parse --show-toplevel").toString().trim();
  const ants: string[] = readFileSync(join(root, "ants.txt"), {
    encoding: "utf-8",
  })
    .toString()
    .trim()
    .split("\n");

  const min = 0;
  const max = Math.pow(2, 31) - 1;

  const difference = Math.abs(max - min);
  const step = Math.floor(difference / ants.length);

  const antHashes = ants.map((ant, i) => {
    const hash: string = (min + i * step).toString();
    return `update ant_release set ant_content_hash = ${hash} where ant_content = '${sanitizeForSql(
      ant
    )}';`;
  });

  const code = hashCode(antHashes.join("\n"));

  const sql = `BEGIN;

${antHashes.join("\n")}

${migrationSql(`reorder-content-hash-to-match-legacy:${code}`)}
  
COMMIT;`;

  console.log(sql);
}

main();
