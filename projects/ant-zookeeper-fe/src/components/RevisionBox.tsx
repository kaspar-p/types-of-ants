"use client";

import { ClipboardCopy } from "./ClipboardCopy";
import { color } from "./Pipeline";

export function RevisionBox({
  revs,
  revision,
  failed,
}: {
  revs: string[];
  revision: string | undefined;
  failed?: boolean;
}) {
  const c = color(revs, revision);

  return (
    <ClipboardCopy text={revision}>
      <span
        className={`
        ${c.text} ${c.bg}
        w-6 h-6 flex justify-center items-center border border-black
        rounded ${revision ? "cursor-pointer" : ""}
        ${failed ? "bg-red-700" : ""}
        `}
      >
        <div>{c.i}</div>
      </span>
    </ClipboardCopy>
  );
}
