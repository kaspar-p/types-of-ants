"use client";

import { createContext, PropsWithChildren } from "react";

type DatePreference = "local-relative" | "utc-absolute";

export const DatePreferenceContext = createContext<DatePreference | undefined>(
  "local-relative",
);

export default function DatePreferenceProvider({
  children,
}: PropsWithChildren<object>) {
  return (
    <DatePreferenceContext value={"local-relative"}>
      {children}
    </DatePreferenceContext>
  );
}
