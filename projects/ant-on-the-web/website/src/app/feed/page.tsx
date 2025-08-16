"use client";

import InputBanner from "@/components/InputBanner";
import { NewsletterBox } from "@/components/NewsletterBox";
import { SuggestionBox } from "@/components/SuggestionBox";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { action } from "@/server/posts";
import { Ant, getUnseenAnts } from "@/server/queries";
import { UserContext } from "@/state/userContext";
import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import React, { useContext, useEffect, useState } from "react";

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
        <Link href={`/${ant.createdByUsername}`}>@{ant.createdByUsername}</Link>{" "}
        <small className="pl-1">{formatDate(ant.createdAt)}</small>
      </div>
      <div className="pl-4">{ant.antName}</div>
    </div>
  );
}

export default function Feed() {
  const [page] = useState(0);

  const {
    isLoading,
    isError,
    data: unseenAnts,
    refetch,
  } = useQuery({
    queryKey: ["unseenAnts"],
    queryFn: () => getUnseenAnts(page),
    refetchInterval: 10_000,
  });

  return (
    <ErrorBoundary isError={isError}>
      <LoadingBoundary isLoading={isLoading}>
        <div>
          <InputBanner
            onSuggestion={async () => {
              await refetch();
            }}
          />

          <h3>
            latest ant submissions ({unseenAnts?.length}):{" "}
            <button
              id="feed-refresh"
              onClick={() => {
                action({
                  action: "click",
                  targetType: "button",
                  target: "feed-refresh",
                });
                refetch();
              }}
            >
              refresh
            </button>
          </h3>
          {unseenAnts?.map((ant, i) => (
            <AntPost key={i} ant={ant} />
          ))}
        </div>
      </LoadingBoundary>
    </ErrorBoundary>
  );
}
