"use client";

import React, { useEffect, useState } from "react";
import { AntBanner } from "../components/AntBanner";
import { escapeAnt } from "../utils/utils";
import { getReleasedAnts } from "../server/queries";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { SuggestionBox } from "../components/SuggestionBox";
import { NewsletterBox } from "@/components/NewsletterBox";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { TUserContext, UserContext } from "../state/userContext";
import InputBanner from "@/components/InputBanner";

export default function Home() {
  const [user, setUser] = useState<TUserContext>({ weakAuth: false });

  const {
    isLoading,
    isError,
    data: releasedAnts,
    fetchNextPage,
    hasNextPage,
  } = useInfiniteQuery({
    queryKey: ["releasedAnts"],
    queryFn: (ctx) => getReleasedAnts(ctx.pageParam ?? 0),
    getNextPageParam: (receivedPage, allPages) =>
      receivedPage.hasNextPage ? allPages.length : undefined,
    keepPreviousData: true,
  });

  useEffect(() => {
    if (hasNextPage) fetchNextPage();
  });

  return (
    <ErrorBoundary isError={isError}>
      <LoadingBoundary isLoading={isLoading}>
        <UserContext.Provider value={{ setUser, user }}>
          <InputBanner />
          <AntBanner />

          <div className="pb-0" id="ant-filler">
            {releasedAnts?.pages.map((page, pageNum) =>
              page.ants.map((ant, i) => (
                <div key={pageNum * 1000 + i}>{escapeAnt(ant)}</div>
              ))
            )}
          </div>
        </UserContext.Provider>
      </LoadingBoundary>
    </ErrorBoundary>
  );
}
