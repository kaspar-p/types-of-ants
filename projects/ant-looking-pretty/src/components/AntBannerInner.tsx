"use client";

import { PropsWithChildren, useState } from "react";
import Marquee from "react-fast-marquee";

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

export function AntBannerInner({
  bannerAnts,
}: PropsWithChildren<{ bannerAnts?: { date: Date; ants: string[] } }>) {
  const [scroll, setScroll] = useState<boolean>(true);

  return (
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
  );
}
