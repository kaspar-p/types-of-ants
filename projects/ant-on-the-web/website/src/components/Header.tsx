"use client";

import { getLatestRelease, getTotalAnts } from "@/server/queries";
import { useQuery } from "@tanstack/react-query";
import React, { useContext } from "react";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { useRouter } from "next/navigation";
import { TUserContext, UserContext } from "@/state/userContext";
import { logout } from "@/server/posts";

export function Header() {
  const { user, setUser } = useContext(UserContext);

  const { push } = useRouter();

  const totalAntsResult = useQuery({
    queryKey: ["totalAnts"],
    queryFn: () => getTotalAnts(),
  });

  const latestReleaseResult = useQuery({
    queryKey: ["releaseNumber"],
    queryFn: getLatestRelease,
  });

  const handleLogout = async () => {
    await logout();
    setUser({ loggedIn: false });
  };

  return (
    <ErrorBoundary
      isError={totalAntsResult.isError || latestReleaseResult.isError}
    >
      <LoadingBoundary
        isLoading={totalAntsResult.isLoading || latestReleaseResult.isLoading}
      >
        <div className="p-5" style={{ fontFamily: "serif" }}>
          <div className="flex flex-row align-center justify-center">
            <h1 className="mb-0 pb-5">
              types of ants{" "}
              <span className="text-sm">
                v{latestReleaseResult.data?.release.releaseNumber}
              </span>
            </h1>
          </div>
          <h3 className="text-center m-0">
            ants discovered to date: {totalAntsResult?.data}
          </h3>
          <div className="flex flex-col space-y-2 py-4 max-w-md mx-auto">
            <div className="text-center flex flex-row space-x-2 align-center justify-center">
              {!user.loggedIn && (
                <button onClick={() => push("/login")}>log in / signup</button>
              )}
              <button onClick={() => push("/")}>home</button>
              {user.loggedIn && (
                <>
                  <button onClick={() => push("/profile")}>profile</button>
                  <button onClick={() => push("/feed")}>feed</button>
                </>
              )}
              <button onClick={() => push("/info")}>contact me</button>
              <button
                onClick={() =>
                  push("https://www.github.com/kaspar-p/types-of-ants")
                }
              >
                read the code
              </button>
              {user.loggedIn && (
                <button onClick={() => handleLogout()}>logout</button>
              )}
            </div>
          </div>
        </div>
      </LoadingBoundary>
    </ErrorBoundary>
  );
}
