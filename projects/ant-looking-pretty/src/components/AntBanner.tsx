import React, { useState } from "react";
import Marquee from "react-fast-marquee";
import { getLatestAnts } from "../server/queries";
import { useQuery } from "@tanstack/react-query";
import { ErrorBoundary, LoadingBoundary } from "./UnhappyPath";
import { webAction } from "@/server/posts";

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

export function AntBanner() {
  const [scroll, setScroll] = useState<boolean>(true);

  const {
    isLoading,
    isError,
    data: bannerAnts,
  } = useQuery({
    queryKey: ["bannerAnts"],
    queryFn: getLatestAnts,
  });

  return (
    <ErrorBoundary isError={isError}>
      <LoadingBoundary isLoading={isLoading}>
        <div
          className="block pr-0 rounded-md p-3"
          style={{ backgroundColor: "gold" }}
          onMouseEnter={() => setScroll(false)}
          onMouseLeave={() => setScroll(true)}
        >
          <>
            <div>
              discovered {bannerAnts?.ants.length} new ant
              {(bannerAnts?.ants.length ?? 0) > 1 ? "s" : ""} on{" "}
              {formatDate(bannerAnts?.date || new Date())}:
            </div>
            <Marquee
              autoFill
              play={scroll}
              speed={75}
              className="items-center flex justify-between"
            >
              {bannerAnts?.ants.map((ant, i) => (
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
        </div>
      </LoadingBoundary>
    </ErrorBoundary>
  );
}
