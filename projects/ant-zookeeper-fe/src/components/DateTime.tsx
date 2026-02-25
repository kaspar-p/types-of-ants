"use client";

import { DatePreferenceContext } from "@/context/DatePreference";
import { useContext } from "react";

const pad = (s: string, padTo: number, padWith: string): string => {
  if (s.length >= padTo) return s;

  return (
    Array.from({ length: padTo - s.length })
      .map(() => padWith)
      .join("") + s
  );
};

function DateTimeText({
  pref,
  date,
}: {
  pref: "local-relative" | "utc-absolute";
  date: Date;
}) {
  switch (pref) {
    case "local-relative": {
      const diff = new Date().getTime() - date.getTime();

      const secs = Math.floor(diff / 1000);
      const mins = Math.floor(diff / (1000 * 60));
      const hours = Math.floor(diff / (1000 * 60 * 60));
      const days = Math.floor(diff / (1000 * 60 * 60 * 24));

      if (mins === 0) {
        return <div>{secs}s ago</div>;
      }

      if (mins < 5) {
        return (
          <div>
            {mins}m {secs % 60}s ago
          </div>
        );
      }

      if (hours === 0) {
        return <div>{mins}m ago</div>;
      }

      if (days === 0) {
        return (
          <div>
            {hours}h {mins % 60}m ago
          </div>
        );
      }

      return (
        <div>
          {days}d {hours % 24}h ago
        </div>
      );
    }

    case "utc-absolute": {
      const time = `${pad(date.getHours().toString(), 2, "0")}:${pad(date.getMinutes().toString(), 2, "0")}:${pad(date.getSeconds().toString(), 2, "0")}`;
      const dateStr = `${date.getFullYear()}-${pad((date.getMonth() + 1).toString(), 2, "0")}-${pad(date.getDate().toString(), 2, "0")}`;

      return (
        <div className="flex flex-row space-x-1 ">
          <div>@</div>
          <div>{time}</div>
          <div>{dateStr}</div>
        </div>
      );
    }
  }
}

export function DateTime(props: { date: Date | string }) {
  const pref = useContext(DatePreferenceContext);
  if (!pref) throw "!";

  const date = new Date(props.date);

  return (
    <div className="bg-gray-100 rounded px-1 py-0.5">
      <DateTimeText pref={pref} date={date} />
    </div>
  );
}
