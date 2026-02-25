"use client";

import { useEffect, useRef, useState } from "react";
import { DateTime } from "./DateTime";
import { refresh } from "@/server/actions";

export function RefreshCounter() {
  const [refreshTime, setRefreshTime] = useState(new Date());

  useInterval(() => {
    refresh();
    setRefreshTime(new Date());
  }, 10_000);

  return (
    <div className="flex flex-row space-x-4">
      <div className="flex flex-row space-x-2 items-center">
        <div>last refreshed</div>
        <DateTime date={refreshTime} />
      </div>

      <button
        onClick={async () => {
          await refresh();
          setRefreshTime(new Date());
        }}
      >
        refresh
      </button>
    </div>
  );
}

type Fn = () => unknown;
function useInterval(callback: Fn, delay: number): void {
  const savedCallback = useRef<Fn | undefined>(undefined);

  // Remember the latest callback.
  useEffect(() => {
    savedCallback.current = callback;
  }, [callback]);

  // Set up the interval.
  useEffect(() => {
    function tick() {
      savedCallback.current?.();
    }
    if (delay !== null) {
      const id = setInterval(tick, delay);
      return () => clearInterval(id);
    }
  }, [delay]);
}
