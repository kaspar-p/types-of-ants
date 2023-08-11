import {
  getAllLegacyAnts,
  getAllDeclinedAndAcceptedAnts,
  getSiteAntFromContent,
  AcceptedAnt,
  LegacyAnt,
} from "./main";
import { getSiteData } from "./parse";
import * as path from "path";
import * as fs from "fs-extra";

function allAcceptedAntsAreSiteAnts(
  acceptedAnts: AcceptedAnt[],
  siteAnts: Set<string>
) {
  console.log("Checking that all accepted ants are site ants...");
  acceptedAnts.forEach((ant) => {
    const acceptedAnt = getSiteAntFromContent(ant.antContent);
    if (!acceptedAnt) {
      console.error(`Accepted ant '${ant.antContent}' not found as site ant!`);
    } else {
      siteAnts.delete(ant.antContent);
      siteAnts.delete(ant.originalSuggestionContent);
    }
  });
}

function allLegacyAntsAreSiteAnts(
  legacyAnts: LegacyAnt[],
  siteAnts: Set<string>
) {
  console.log("Checking that all legacy ants are site ants...");
  legacyAnts.forEach((ant) => {
    const siteAntContent = getSiteAntFromContent(ant.antContent);
    if (!siteAntContent) {
      console.error(
        `Legacy ant '${ant.antContent}' not found as site ant content!`
      );
    } else {
      siteAnts.delete(ant.antContent);
      siteAnts.delete(ant.originalSuggestionContent);
    }
  });
}

function allLegacyAntsAreSiteAntsWithOriginalSuggestionContent(
  legacyAnts: LegacyAnt[],
  siteAnts: Set<string>
) {
  console.log(
    "Checking that all legacy ants are site ants with their original content..."
  );
  legacyAnts.forEach((ant) => {
    const siteAntOriginal = getSiteAntFromContent(
      ant.originalSuggestionContent
    );
    if (!siteAntOriginal) {
      console.error(
        `Legacy ant '${ant.antContent}' not found as site ant original!`
      );
    } else {
      siteAnts.delete(ant.antContent);
      siteAnts.delete(ant.originalSuggestionContent);
    }
  });
}

function allAcceptedAntsAreInAntsTxt(
  acceptedAnts: AcceptedAnt[],
  txtAnts: Set<string>
) {
  console.log("Checking that all accepted ants are in ants.txt...");

  acceptedAnts.forEach((ant) => {
    if (
      !txtAnts.has(ant.antContent) &&
      !txtAnts.has(ant.originalSuggestionContent)
    ) {
      console.error(
        `Ant (${ant.antContent}, ${ant.originalSuggestionContent}) not in ants.txt!`
      );
    }
  });
}

export function allAntsTxtAntsAreAcceptedAndSiteAnts(
  acceptedAnts: AcceptedAnt[],
  txtAnts: Set<string>,
  legacyAnts: LegacyAnt[]
) {
  const acceptedSetContent = new Set(acceptedAnts.map((a) => a.antContent));
  const acceptedSetOriginal = new Set(
    acceptedAnts.map((a) => a.originalSuggestionContent)
  );
  const legacySet = new Set(legacyAnts.map((a) => a.antContent));

  Array.from(txtAnts).forEach((ant) => {
    const accepted =
      acceptedSetContent.has(ant) || acceptedSetOriginal.has(ant);
    const legacy = legacySet.has(ant);
    const siteAnt = getSiteAntFromContent(ant) !== undefined;
    if (!accepted && !legacy && siteAnt) {
      console.error(
        `Txt ant ${ant} not accepted and not legacy! Need to add 'suggestedContent' to it!`
      );
    }
    if (!siteAnt) {
      console.error(
        `Txt ant ${ant} not site ant! Add it to the site ants file!`
      );
    }
  });
}

export async function checkIntegrity(
  siteDataFile: string,
  rawDataFile: string
) {
  console.log("Running...");
  const legacyAnts = await getAllLegacyAnts(siteDataFile);
  console.log("Got all legacy ants!");
  const { acceptedAnts, declinedAnts } = await getAllDeclinedAndAcceptedAnts(
    rawDataFile
  );
  console.log("Got all accepted and declined ants!");
  const siteAnts: Set<string> = new Set(
    getSiteData(siteDataFile).ants.map((ant) => ant.ant)
  );
  const antsTxtContent = fs
    .readFileSync(path.join(__dirname, "..", "..", "..", "old", "ants.txt"), {
      encoding: "utf8",
    })
    .trim();
  let txtAnts = new Set(antsTxtContent.split("\n").map((s) => s.trim()));

  allAntsTxtAntsAreAcceptedAndSiteAnts(acceptedAnts, txtAnts, legacyAnts);
  allAcceptedAntsAreInAntsTxt(acceptedAnts, txtAnts);
  allAcceptedAntsAreSiteAnts(acceptedAnts, siteAnts);
  allLegacyAntsAreSiteAnts(legacyAnts, siteAnts);
  allLegacyAntsAreSiteAntsWithOriginalSuggestionContent(legacyAnts, siteAnts);

  console.log("Finished: ", siteAnts.size, Array.from(siteAnts));
}
