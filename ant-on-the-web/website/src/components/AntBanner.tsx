import React from "react";
import Marquee from "react-fast-marquee";
import { useQuery } from "@/utils/useQuery";
import { getLatestAnts } from "../server/queries";

const defaultAnts = [
  "silly ant",
  "vampire ant (mosquito)",
  "sad ant",
  "prisoner ant",
  "ant with daddy issues",
  "ant on the ceiling (gonna fall down)",
  "ant but dead",
  "arsonist ant",
  "tired ant",
  "uncle ant",
  "ant on a plane",
  "ant selling wares",
  "ant so long it looks weird",
  "ant graduating from college",
  "ant who made it to the top floor but found 0 crumbs and doesn't know how to get home",
  '"ant"',
  "official ant",
  "lumpy ant",
  "mac ant",
  "caged ant",
  "ant that has a bone to pick with you",
  "ant that supplies ideas for this website",
  "ant in a uhaul",
  "they/them ant",
  "windows ant",
  "chill ant",
  "ant on parole",
  "ant at a funeral",
];

function formatDate(d: Date): string {
  const months = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
  ];
  return `${months[d.getMonth()]} ${d.getDate()}, ${d.getFullYear()}`;
}

type AntBannerProps = {};
export function AntBanner(props: AntBannerProps) {
  const {
    res: latestAnts,
    loading: latestAntsLoading,
    err: latestAntsError,
  } = useQuery(getLatestAnts);

  return (
    <div
      className="block w-full pr-0"
      style={{
        backgroundColor: "gold",
        borderRadius: "6px",
        padding: "12px",
      }}
    >
      {latestAntsLoading ? (
        "Loading..."
      ) : latestAntsError || !latestAnts ? (
        "ERROR"
      ) : (
        <>
          <div>
            discovered {latestAnts.ants.length} new ants on{" "}
            {formatDate(latestAnts.date)}:
          </div>
          <Marquee
            autoFill
            pauseOnHover
            speed={75}
            className="items-center flex justify-between"
          >
            {latestAnts.ants.map((ant, i) => (
              <div
                key={i}
                className="inline-block whitespace-nowrap pr-4 pl-4"
                style={{ paddingRight: "15px", paddingLeft: "15px" }}
              >
                {ant}
              </div>
            ))}
          </Marquee>
        </>
      )}
    </div>
  );
}
