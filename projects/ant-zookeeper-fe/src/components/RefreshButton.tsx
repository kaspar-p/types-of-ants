"use client";

import { refresh } from "@/server/actions";
import { PropsWithChildren } from "react";

export const RefreshButton = (props: PropsWithChildren<{}>) => {
  "use client";

  return <button onClick={() => refresh()}>{props.children}</button>;
};
