"use client";

import { isServer } from "@tanstack/react-query";
import { useMediaQuery } from "usehooks-ts";
import { AntText } from "./AntText";
import { ReleasedAnt } from "@/server/queries";
import { Suspense } from "react";

type AntColumnsProps = {
  ants: ReleasedAnt[];
};

export function AntColumns(props: AntColumnsProps) {
  const isSmallDevice = isServer
    ? false
    : useMediaQuery("only screen and (max-width : 768px)");
  const isMediumDevice = isServer
    ? true
    : useMediaQuery(
        "only screen and (min-width : 769px) and (max-width : 992px)",
      );
  const isLargeDevice = isServer
    ? false
    : useMediaQuery(
        "only screen and (min-width : 993px) and (max-width : 1200px)",
      );

  let columns: number;
  if (isSmallDevice) {
    columns = 1;
  } else if (isMediumDevice) {
    columns = 2;
  } else if (isLargeDevice) {
    columns = 3;
  } else {
    columns = 5;
  }

  const antColumns: ReleasedAnt[][] = [];
  const numAntsInColumn = props.ants.length / columns;
  for (let i = 0; i < columns; i++) {
    antColumns.push(
      props.ants.slice(i * numAntsInColumn, (i + 1) * numAntsInColumn),
    );
  }

  return (
    <Suspense fallback="loading...">
      <div className="mt-2">
        <div className="flex flex-row gap-2 justify-center">
          {antColumns.map((ants, c) => (
            <div key={c} className="flex flex-col gap-1 justify-start">
              {ants.map((ant, i) => (
                <AntText key={i} ant={ant} />
              ))}
            </div>
          ))}
        </div>
      </div>
    </Suspense>
  );
}
