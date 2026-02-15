"use client";

import { webAction } from "@/server/posts";

export function RefreshButton(props: { onRefresh?: () => void }) {
  return (
    <button
      id="feed-refresh"
      onClick={() => {
        webAction({
          action: "click",
          targetType: "button",
          target: "feed-refresh",
        });
        props.onRefresh?.();
      }}
    >
      refresh
    </button>
  );
}
