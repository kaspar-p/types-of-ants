"use client";

import React, { useState } from "react";
import { AntBanner } from "../components/AntBanner";
import { escapeAnt } from "../utils/utils";
import { getReleasedAnts } from "../server/queries";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { SuggestionBox } from "../components/SuggestionBox";
import { NewsletterBox } from "@/components/NewsletterBox";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";

export default function Home() {
  const [page, setPage] = useState(0);

  const {
    isLoading,
    isError,
    data: releasedAnts,
  } = useQuery({
    queryKey: ["releasedAnts"],
    queryFn: () => getReleasedAnts(page),
  });

  return (
    <ErrorBoundary isError={isError}>
      <LoadingBoundary isLoading={isLoading}>
        <div
          id="forms-container"
          style={{
            display: "flex",
            flexDirection: "row",
            flexWrap: "wrap",
            alignSelf: "center",
          }}
        >
          <SuggestionBox />
          <NewsletterBox />
        </div>
        <AntBanner />
        <div id="ant-filler">
          {releasedAnts?.ants.map((ant, i) => (
            <div key={i}>{escapeAnt(ant)}</div>
          ))}
        </div>
      </LoadingBoundary>
    </ErrorBoundary>
  );
}
