"use client";

import { getReleasedAnts, getReleaseNumber } from "@/server/queries";
import { useQuery } from "@tanstack/react-query";
import React, { useState } from "react";
import { errorOr } from "@/components/UnhappyPath";
import { useRouter } from "next/navigation";

type InnerProps = {
  releaseNumber: number;
  allAnts: { ants: string[] };
  children: React.ReactNode;
};
function Inner({ allAnts, releaseNumber, children }: InnerProps) {}

type HeaderProps = {
  children: React.ReactNode;
};
export function Header({ children }: HeaderProps) {
  const [page, setPage] = useState(0);
  const { push } = useRouter();

  const allAntsResult = useQuery({
    queryKey: ["allAnts"],
    queryFn: () => getReleasedAnts(page),
  });

  const releaseNumberResult = useQuery({
    queryKey: ["releaseNumber"],
    queryFn: getReleaseNumber,
  });

  return errorOr(
    [allAntsResult.isLoading, releaseNumberResult.isLoading],
    [allAntsResult.isError, releaseNumberResult.isError],
    { allAnts: allAntsResult.data, releaseNumber: releaseNumberResult.data },
    ({ allAnts, releaseNumber }) => (
      <div style={{ padding: "20px", fontFamily: "serif" }}>
        <h1 className="title">
          types of ants{" "}
          <span style={{ fontSize: "12pt" }}>v{releaseNumber}</span>
        </h1>
        <h2 className="title">
          ants discovered to date: {allAnts.ants.length}
        </h2>{" "}
        <h3 className="title w-full flex flex-row space-x-2 align-center justify-center">
          <button onClick={() => push("/")}>home</button>
          <button onClick={() => push("/feed")}>feed</button>
          <button onClick={() => push("/info")}>contact me</button>
          <button
            onClick={() =>
              push("https://www.github.com/kaspar-p/types-of-ants")
            }
          >
            read the code
          </button>
        </h3>
        {children}
      </div>
    )
  );
}
