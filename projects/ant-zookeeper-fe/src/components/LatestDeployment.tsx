import { DateTime } from "./DateTime";
import { Revision } from "./Pipeline";
import { RevisionBox } from "./RevisionBox";

export function LatestDeployment(props: {
  revisions: string[];
  finished: Revision | undefined;
}) {
  return (
    <span className="flex flex-row space-x-1 items-center">
      <RevisionBox revs={props.revisions} revision={props.finished?.revision} />
      <div className="flex flex-row space-x-1 items-center">
        <div>latest</div>
        {props.finished ? (
          <DateTime date={props.finished.reachedAt} />
        ) : undefined}
      </div>
    </span>
  );
}
