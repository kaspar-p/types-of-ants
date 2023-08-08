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
  const [loggedIn, setLoggedIn] = useState(false);

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
      <div className="p-5" style={{ fontFamily: "serif" }}>
        <div className="flex flex-row align-center justify-center">
          <h1 className="mb-0 pb-5">
            types of ants <span className="text-sm">v{releaseNumber}</span>
          </h1>
        </div>
        <h3 className="text-center m-0">
          ants discovered to date: {allAnts.ants.length}
        </h3>
        <div className="flex flex-col space-y-2 py-4 max-w-md mx-auto">
          <div className="text-center flex flex-row space-x-2 align-center">
            {loggedIn ? (
              <button onClick={() => push("/profile")}>profile</button>
            ) : (
              <button onClick={() => push("/login")}>log in / signup</button>
            )}
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
          </div>
        </div>
        {children}
      </div>
    )
  );
}
