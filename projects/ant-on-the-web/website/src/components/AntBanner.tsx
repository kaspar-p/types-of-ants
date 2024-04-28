import React from "react";
import Marquee from "react-fast-marquee";
import { getLatestAnts } from "../server/queries";
import { useQuery } from "@tanstack/react-query";
import { ErrorBoundary, LoadingBoundary } from "./UnhappyPath";

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
          className="block w-full pr-0"
          style={{
            backgroundColor: "gold",
            borderRadius: "6px",
            padding: "12px",
          }}
        >
          <>
            <div>
              discovered {bannerAnts?.ants.length} new ants on{" "}
              {formatDate(bannerAnts?.date || new Date())}:
            </div>
            <Marquee
              autoFill
              pauseOnHover
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
