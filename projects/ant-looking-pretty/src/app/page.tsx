"use client";

import { useEffect } from "react";
import { AntBanner } from "../components/AntBanner";
import { getReleasedAnts, ReleasedAnt } from "../server/queries";
import { useInfiniteQuery } from "@tanstack/react-query";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { InputBanner } from "@/components/InputBanner";
import { AntText } from "@/components/AntText";
import { useMediaQuery } from "usehooks-ts";

const IS_SERVER = typeof window === "undefined";

export default function Home() {
  const {
    isLoading,
    isError,
    data: releasedAnts,
    fetchNextPage,
    hasNextPage,
  } = useInfiniteQuery({
    initialPageParam: 0,
    queryKey: ["releasedAnts"],
    queryFn: async (ctx) => getReleasedAnts(ctx.pageParam),
    getNextPageParam: (receivedPage, allPages) => {
      return receivedPage.hasNextPage ? allPages.length : undefined;
    },
    placeholderData: { pageParams: [], pages: [] },
  });

  useEffect(() => {
    if (hasNextPage) fetchNextPage();
  });

  const isSmallDevice = useMediaQuery("only screen and (max-width : 768px)");
  const isMediumDevice = useMediaQuery(
    "only screen and (min-width : 769px) and (max-width : 992px)"
  );
  const isLargeDevice = useMediaQuery(
    "only screen and (min-width : 993px) and (max-width : 1200px)"
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

  const ants = releasedAnts?.pages.flatMap((page) => page.ants) ?? [];
  const antColumns: ReleasedAnt[][] = [];
  const numAntsInColumn = ants.length / columns;
  for (let i = 0; i < columns; i++) {
    antColumns.push(ants.slice(i * numAntsInColumn, (i + 1) * numAntsInColumn));
  }

  return (
    <ErrorBoundary isError={isError}>
      <LoadingBoundary isLoading={isLoading}>
        <InputBanner />
        <AntBanner />

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
      </LoadingBoundary>
    </ErrorBoundary>
  );
}
