"use client";

import { getTotalAnts, getVersion } from "@/server/queries";
import { useQuery } from "@tanstack/react-query";
import React, { useContext } from "react";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { useRouter } from "next/navigation";
import { UserContext } from "@/state/userContext";
import { logout } from "@/server/posts";
import Link from "next/link";

export function Header() {
  const { user, setUser } = useContext(UserContext);

  const { push } = useRouter();

  const totalAntsResult = useQuery({
    queryKey: ["totalAnts"],
    queryFn: () => getTotalAnts(),
  });

  const version = useQuery({
    queryKey: ["version"],
    queryFn: getVersion,
  });

  const handleLogout = async () => {
    await logout();
    setUser({ weakAuth: false });
  };

  return (
    <ErrorBoundary isError={totalAntsResult.isError || version.isError}>
      <LoadingBoundary
        isLoading={totalAntsResult.isLoading || version.isLoading}
      >
        <div className="p-5" style={{ fontFamily: "serif" }}>
          <div className="flex flex-row align-center justify-center">
            <h1 className="mb-0 pb-5">
              types of ants <span className="text-sm">v1.{version.data}</span>
            </h1>
          </div>
          <h3 className="text-center m-0">
            ants discovered to date: {totalAntsResult?.data}
          </h3>
          <div className="flex flex-col space-y-2 py-4 max-w-md mx-auto">
            <div className="text-center flex flex-row space-x-2 align-center justify-center">
              {!(user.weakAuth && user.loggedIn) && (
                <button
                  className="cursor-pointer"
                  onClick={() => push("/login")}
                >
                  log in / signup
                </button>
              )}
              <button className="cursor-pointer" onClick={() => push("/")}>
                home
              </button>
              {user.weakAuth && user.loggedIn && (
                <>
                  <button
                    className="cursor-pointer"
                    onClick={() => push("/profile")}
                  >
                    profile
                  </button>
                  <button
                    className="cursor-pointer"
                    onClick={() => push("/feed")}
                  >
                    feed
                  </button>
                </>
              )}
              <button className="cursor-pointer" onClick={() => push("/info")}>
                contact me
              </button>

              {user.weakAuth && user.loggedIn && (
                <button
                  className="cursor-pointer"
                  onClick={() => handleLogout()}
                >
                  logout
                </button>
              )}
            </div>
            <div className="text-center flex flex-row space-x-2 align-center justify-center">
              <Link href="https://twitter.com/typesofants">
                <button className="cursor-pointer">twitter @typesofants</button>
              </Link>
              <Link href="https://github.com/kaspar-p/types-of-ants">
                <button className="cursor-pointer">
                  ant who wants to read the code
                </button>
              </Link>
            </div>
          </div>
        </div>
      </LoadingBoundary>
    </ErrorBoundary>
  );
}
