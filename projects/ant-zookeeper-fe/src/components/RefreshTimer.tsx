"use client";

import { refresh } from "@/server/actions";
import { useState } from "react";
import { useEffect } from "react";
import { useRef } from "react";
import { formatDatetime } from "./Pipeline";

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

export function RefreshTimer() {
  const [refreshTime, setRefreshTime] = useState(new Date());

  useInterval(() => {
    refresh();
    setRefreshTime(new Date());
  }, 10_000);

  return <div>last refreshed at: {formatDatetime(refreshTime)}</div>;
}
