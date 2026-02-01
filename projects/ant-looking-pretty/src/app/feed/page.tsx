"use client";

import { InputBanner } from "@/components/InputBanner";
import { ErrorBoundary, LoadingBoundary } from "@/components/UnhappyPath";
import { webAction } from "@/server/posts";
import { Ant, getUnseenAnts, unwrap } from "@/server/queries";
import { useQuery } from "@tanstack/react-query";
import Link from "next/link";

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
    <div className="p-1.5">
      <div>
        <Link href={`/im/${ant.createdByUsername}`}>
          @{ant.createdByUsername}
        </Link>{" "}
        <small className="pl-1">{formatDate(ant.createdAt)}</small>
      </div>
      <div className="pl-4">{ant.antName}</div>
    </div>
  );
}

export default function FeedPage() {
  const {
    isLoading,
    isError,
    data: unseenAnts,
    refetch,
  } = useQuery({
    queryKey: ["unseenAnts"],
    queryFn: () => unwrap(getUnseenAnts(0)),
    refetchInterval: 10_000,
  });

  return (
    <div>
      <InputBanner
        onSuggestion={async () => {
          await refetch();
        }}
      />

      <h3 className="mb-1">
        latest ant submissions ({unseenAnts?.length ?? 0}):{" "}
        <button
          id="feed-refresh"
          onClick={() => {
            webAction({
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

      <ErrorBoundary isError={isError}>
        <LoadingBoundary isLoading={isLoading}>
          {unseenAnts?.map((ant, i) => (
            <AntPost key={i} ant={ant} />
          ))}
        </LoadingBoundary>
      </ErrorBoundary>
    </div>
  );
}
