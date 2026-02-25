import { ReactNode } from "react";
import { DateTime } from "./DateTime";
import { Revision } from "./Pipeline";
import { RevisionBox } from "./RevisionBox";

type InProgressDeploymentsProps = {
  revisions: string[];
  inProgress: Revision[];
  verb?: ReactNode;
};

export function InProgressDeployments(props: InProgressDeploymentsProps) {
  return (
    <>
      {props.inProgress.length > 0 ? (
        <div>
          {props.inProgress.map((r, i) => (
            <div key={i}>
              <span className="flex flex-row items-center space-x-1">
                <RevisionBox
                  revs={props.revisions}
                  revision={props.inProgress[0].revision}
                />
                <div>{props.verb ? props.verb : "in progress"}, started</div>
                <DateTime date={r.reachedAt} />
              </span>
            </div>
          ))}
        </div>
      ) : undefined}
    </>
  );
}
