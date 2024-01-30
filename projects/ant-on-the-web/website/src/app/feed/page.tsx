"use client";

import { NewsletterBox } from "@/components/NewsletterBox";
import { SuggestionBox } from "@/components/SuggestionBox";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { Ant, getUnseenAnts } from "@/server/queries";
import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import React, { useState } from "react";

function formatDate(createdUtcMilliseconds: string): string {
  const months = [
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
  ];
  const d = new Date(createdUtcMilliseconds);

  const date = `${months[d.getMonth()]} ${d.getDate()} ${d.getFullYear()}`;
  const minutes =
    d.getMinutes().toString().length === 1
      ? "0" + d.getMinutes().toString()
      : d.getMinutes().toString();
  return `${d.getHours()}:${minutes}, ${date}`;
}

export type AntPostProps = {
  ant: Ant;
};

function AntPost({ ant }: AntPostProps) {
  return (
    <div className="p-2 border-black border-b-2">
      <div>
        <Link href="/nobody">@nobody</Link>{" "}
        <small className="pl-1">{formatDate(ant.created_at)}</small>
      </div>
      <div className="pl-4">{ant.ant_name}</div>
    </div>
  );
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
                await refetch();
              }}
            />
            <NewsletterBox />
          </div>
          <h3>
            latest ant submissions ({unseenAnts?.length}):{" "}
            <button onClick={() => refetch()}>refresh</button>
          </h3>
          {unseenAnts?.map((ant, i) => (
            <AntPost key={i} ant={ant} />
          ))}
        </div>
      </LoadingBoundary>
    </ErrorBoundary>
  );
}
