"use client";

import { getReleasedAnts, getLatestRelease, getUser } from "@/server/queries";
import { useQuery } from "@tanstack/react-query";
import React, { useEffect, useState } from "react";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { useRouter } from "next/navigation";
import { TUserContext, UserContext } from "@/state/userContext";
import { logout } from "@/server/posts";

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
  const [user, setUser] = useState<TUserContext>({ loggedIn: false });

  const { push } = useRouter();

  const allAntsResult = useQuery({
    queryKey: ["allAnts"],
    queryFn: () => getReleasedAnts(page),
  });

  const latestReleaseResult = useQuery({
    queryKey: ["releaseNumber"],
    queryFn: getLatestRelease,
  });

  const latestRelease = latestReleaseResult.data;
  const allAnts = allAntsResult.data;

  useEffect(() => {
    async function checkLoggedIn() {
      const res = await getUser();
      if (res.ok) {
        setUser({ loggedIn: true, user: (await res.json()).user });
      }
    }

    checkLoggedIn();
  }, []);

  const handleLogout = async () => {
    await logout();
    setUser({ loggedIn: false });
  };

  return (
    <UserContext.Provider value={{ user, setUser }}>
      <ErrorBoundary
        isError={allAntsResult.isError || latestReleaseResult.isError}
      >
        <LoadingBoundary
          isLoading={allAntsResult.isLoading || latestReleaseResult.isLoading}
        >
          <div className="p-5" style={{ fontFamily: "serif" }}>
            <div className="flex flex-row align-center justify-center">
              <h1 className="mb-0 pb-5">
                types of ants{" "}
                <span className="text-sm">
                  v{latestReleaseResult.data?.release_number}
                </span>
              </h1>
            </div>
            <h3 className="text-center m-0">
              ants discovered to date: {allAnts?.ants.length}
            </h3>
            <div className="flex flex-col space-y-2 py-4 max-w-md mx-auto">
              <div className="text-center flex flex-row space-x-2 align-center">
                {user.loggedIn ? (
                  <>
                    <button onClick={() => push("/profile")}>
                      profile ({user.user.username})
                    </button>
                    <button onClick={() => handleLogout()}>logout</button>
                  </>
                ) : (
                  <button onClick={() => push("/login")}>
                    log in / signup
                  </button>
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
        </LoadingBoundary>
      </ErrorBoundary>
    </UserContext.Provider>
  );
}
