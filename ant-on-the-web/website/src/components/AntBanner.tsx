import React from "react";
import Marquee from "react-fast-marquee";
import { useQuery } from "@/utils/useQuery";
import { getLatestAnts } from "../server/queries";

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
