import { PropsWithChildren } from "react";
import { color, Revision } from "./Pipeline";

export type BoxTitleProps = {
  revisions: string[];
  finished: Revision | undefined;
  inProgress: Revision[];
};

export function BoxTitle(props: PropsWithChildren<BoxTitleProps>) {
  return (
    <div
      className={`
        p-2 border-b border-b-black rounded-t-md flex flex-row space-x-2 items-center
        ${color(props.revisions, props.finished?.revision).text}
        ${color(props.revisions, props.finished?.revision).bg}
        `}
    >
      <div className="flex flex-row space-x-2">{props.children}</div>
    </div>
  );
}
