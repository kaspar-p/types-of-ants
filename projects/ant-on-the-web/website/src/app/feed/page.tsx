"use client";

import { NewsletterBox } from "@/components/NewsletterBox";
import { SuggestionBox } from "@/components/SuggestionBox";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { getUnseenAnts } from "@/server/queries";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import React, { useState } from "react";

function formatDate(createdUtcMilliseconds: string): string {
  const d = new Date(createdUtcMilliseconds);
  return `${d.toLocaleDateString()} ${d.toLocaleTimeString()}`;
}

export default function Feed() {
  const [page, setPage] = useState(0);

  const {
    isLoading,
    isError,
    data: unseenAnts,
    refetch,
  } = useQuery({
    queryKey: ["unseenAnts"],
    queryFn: () => getUnseenAnts(page),
  });

  return (
    <ErrorBoundary isError={isError}>
      <LoadingBoundary isLoading={isLoading}>
        <div>
          <div
            id="forms-container"
            style={{
              display: "flex",
              flexDirection: "row",
              flexWrap: "wrap",
              alignSelf: "center",
            }}
          >
            <SuggestionBox
              action={async () => {
                console.log("before!");
                await refetch();
                console.log("after!");
              }}
            />
            <NewsletterBox />
          </div>
          <h3>
            latest ant submissions ({unseenAnts?.length}):{" "}
            <button onClick={() => refetch()}>refresh</button>
          </h3>
          {unseenAnts?.map((ant, i) => (
            <div key={ant.ant_name + i}>
              [{formatDate(ant.created_at)}] <strong>{ant.ant_name}</strong>
            </div>
          ))}
        </div>
      </LoadingBoundary>
    </ErrorBoundary>
  );
}
