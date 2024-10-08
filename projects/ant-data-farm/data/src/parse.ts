import { z } from "zod";
import * as fs from "fs-extra";

const siteSchema = z.object({
  version: z.string(),
  ants: z.array(
    z.object({
      ant: z.string(),
      tweeted: z.boolean().optional(),
      tweetedAt: z.string().optional(),
      legacy: z.boolean().optional(),
      suggestedContent: z.string().optional(),
    })
  ),
});

export type SiteData = z.infer<typeof siteSchema>;
export function getSiteData(siteDataPath: string): SiteData {
  const content = fs.readFileSync(siteDataPath, {
    encoding: "utf-8",
  });
  const jsonRaw = JSON.parse(content);
  return siteSchema.parse(jsonRaw);
}

const rawSchema = z.array(
  z.object({
    title: z.string().transform((title) => {
      return title.replace(/’/g, "'");
    }),
    state: z.union([z.literal("OPEN"), z.literal("CLOSED")]),
    createdAt: z.string(),
    closed: z.boolean(),
    closedAt: z.union([z.string(), z.null()]),
    body: z.string().transform((title) => {
      return title.replace(/’/g, "'");
    }),
    labels: z.array(
      z.object({
        name: z.string(),
      })
    ),
  })
);
export type RawData = z.infer<typeof rawSchema>;
export async function getRawData(jsonFilePath: string): Promise<RawData> {
  const content = await fs.readFile(jsonFilePath, { encoding: "utf8" });
  const jsonRaw = JSON.parse(content);
  return rawSchema.parse(jsonRaw);
}

// const site = siteSchema.parse(site_data);
// const siteDataMap = site.ants.reduce(
//   (acc, ant) => ({ ...acc, [ant.ant]: ant }),
//   {}
// );

// const titleRegex = /`(.+)`/g;
// const rawSchema = z.array(
//   z.object({
//     title: z.string(),
//     state: z.union([z.literal("OPEN"), z.literal("CLOSED")]),
//     createdAt: z.string(),
//     closed: z.boolean(),
//     closedAt: z.union([z.string(), z.null()]),
//     body: z.string(),
//     labels: z.array(
//       z.object({
//         name: z.string(),
//       })
//     ),
//   })
// );
// const raw = rawSchema.parse(raw_data);
// const issuesAnts = raw
//   .filter((issue) => {
//     if (issue.labels.length === 0) return false;
//     return issue.labels.find((label) => label.name === "autogenerated");
//   })
//   .map((issue) => {
//     const match = issue.title.match(titleRegex);
//     if (!match) throw new Error(`Ant '${issue.title}' failed titleRegex!`);
//     const newTitle = match[0].slice(1, match[0].length - 1);

//     return {
//       ...issue,
//       title: newTitle,
//     };
//   });

// // console.log(onlyAnts);

// const siteAntNames = new Set(site.ants.map((ant) => ant.ant));
// const siteAntNamesOverrides = new Set(
//   site.ants
//     .filter((ant) => !!ant.suggestedContent)
//     .map((ant) => ant.suggestedContent)
// );
// const legacyAntNames = new Set(
//   site.ants.filter((ant) => ant.legacy === true).map((ant) => ant.ant)
// );

// const suggestedContentMap: Record<string, string> = site.ants
//   .filter((ant) => ant.suggestedContent)
//   .reduce(
//     (acc, ant) => ({
//       ...acc,
//       [ant.suggestedContent as unknown as string]: ant.ant,
//     }),
//     {}
//   );

// const deleteFrom = (title: string) => {
//   legacyAntNames.delete(title);
//   siteAntNames.delete(title);
//   siteAntNames.delete(suggestedContentMap[title]);
//   siteAntNamesOverrides.delete(title);
//   siteAntNamesOverrides.delete(suggestedContentMap[title]);
// };

// const confirmedAnts = issuesAnts
//   .filter((ant) => {
//     return ant.state === "CLOSED";
//   })
//   .filter((ant) => {
//     const isLegacy = legacyAntNames.has(ant.title);
//     const has = siteAntNames.has(ant.title);
//     const hasAsOriginalContent = siteAntNamesOverrides.has(ant.title);
//     if (isLegacy || has || hasAsOriginalContent) {
//       deleteFrom(ant.title);
//       return true;
//     }

//     return false;
//   });
// const confirmedAntsSet = new Set(confirmedAnts.map((ant) => ant.title));

// const antsLeft = Array.from(siteAntNames).filter(
//   (name) => !legacyAntNames.has(name)
// );

// const totalAnts = site.ants.length;
// const totalAntsSet = new Set(site.ants);
// const legacyAnts = site.ants.filter((ant) => ant.legacy === true).length;
// const legacyAntsSet = new Set(site.ants.filter((ant) => ant.legacy === true));
// const antNameOverridesSet = new Set(
//   site.ants.filter((ant) => !!ant.suggestedContent).map((ant) => ant.ant)
// );

// console.log("Total ants on the site: ", site.ants.length);
// console.log(
//   "Ants from issuesAnts, in site, verbatim: ",
//   Array.from(confirmedAnts).length
// );
// console.log("Legacy ants (no issue made for them): ", legacyAnts);
// console.log(
//   "total - legacy - confirmed: ",
//   totalAnts - legacyAnts - Array.from(confirmedAnts).length
// );
// console.log("Unaccounted for known ants: ", antsLeft.length);
// console.log("Ants left: ", antsLeft);
// const totalMinusLegacy = new Set(
//   [...totalAntsSet].filter((x) => !legacyAntsSet.has(x)).map((a) => a.ant)
// );
// const totalMinusBoth = new Set(
//   [...totalMinusLegacy].filter((x) => !confirmedAntsSet.has(x))
// );
// const totalMinusThree = new Set(
//   [...totalMinusBoth].filter((x) => !antNameOverridesSet.has(x))
// );
// console.log(
//   "TotalSet - ConfirmedSet - LegacySet - SuggestedContent: ",
//   Array.from(totalMinusThree)
// );
