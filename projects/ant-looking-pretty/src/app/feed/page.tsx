import { InputBanner } from "@/components/InputBanner";
import { ErrorBoundary } from "@/components/UnhappyPath";
import { webAction } from "@/server/posts";
import { Ant, getUnseenAnts } from "@/server/queries";
import { revalidatePath } from "next/cache";
import Link from "next/link";
import { RefreshButton } from "./RefreshButton";
import { Suspense } from "react";

export function formatDate(millis: string): string {
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
  const d = new Date(millis);

  const date = `${months[d.getMonth()]} ${d.getDate()} ${d.getFullYear()}`;

  return date;
}

function formatDatetime(createdUtcMilliseconds: string): string {
  const date = formatDate(createdUtcMilliseconds);

  const d = new Date(createdUtcMilliseconds);
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
        <small className="pl-1">{formatDatetime(ant.createdAt)}</small>
      </div>
      <div className="pl-4">{ant.antName}</div>
    </div>
  );
}

export default async function FeedPage() {
  const unseenAnts = [];
  let nextPage: number | undefined = 0;
  let isError: boolean = false;
  do {
    const res = await getUnseenAnts(nextPage);
    isError = !res.success;

    if (res.success) {
      const data = res.data;
      nextPage = undefined;
      unseenAnts.push(...(data ?? []));
    } else {
      break;
    }
  } while (nextPage !== undefined);

  async function refreshAction() {
    "use server";

    revalidatePath("/");
  }

  return (
    <div>
      <InputBanner onSuggestion={refreshAction} />

      <h3 className="mb-1">
        latest ant submissions ({unseenAnts?.length ?? 0}):{" "}
        <RefreshButton onRefresh={refreshAction} />
      </h3>

      <ErrorBoundary isError={isError}>
        <Suspense fallback="loading...">
          {unseenAnts?.map((ant, i) => (
            <AntPost key={i} ant={ant} />
          ))}
        </Suspense>
      </ErrorBoundary>
    </div>
  );
}
