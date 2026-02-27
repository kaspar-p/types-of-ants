import { BoxTitle } from "./BoxTitle";
import { ClipboardCopy } from "./ClipboardCopy";
import { DateTime } from "./DateTime";
import { InProgressDeployments } from "./InProgressDeployments";
import { LatestDeployment } from "./LatestDeployment";
import { type Host, HostGroup, Progress, revisions } from "./Pipeline";
import { RetryJobButton } from "./RetryJobButton";
import { RevisionBox } from "./RevisionBox";

export type HostProps = {
  index: number;
  total: number;
  hostGroup: HostGroup;
  host: Host;
  progress: Progress;
  revisions: string[];
};

export function Host(props: HostProps) {
  const hostRev = revisions(
    props.progress,
    `${props.hostGroup.hostGroupId}#${props.host.name}`,
  );

  console.log(hostRev);

  return (
    <div>
      <div className="border rounded-lg">
        <BoxTitle
          revisions={props.revisions}
          finished={hostRev.finished}
          inProgress={hostRev.inProgress}
        >
          <span className="flex flex-row space-x-2 items-center">
            <code>{props.host.name}</code>
          </span>
          <div className="text-sm self-center">
            (<i>{props.host.arch}</i>)
          </div>
          <code className="border rounded-md p-1 text-black bg-white">
            host {props.index}/{props.total}
          </code>
        </BoxTitle>

        <div className="p-2 flex flex-col space-y-2">
          {hostRev.failed ? (
            <div className="flex flex-row space-x-2 items-center">
              <RevisionBox
                revs={props.revisions}
                revision={hostRev.failed.revision}
                failed={true}
              />
              <div className="flex flex-col justify-start">
                <div className="text-red-700">failed</div>
                <RetryJobButton jobId={hostRev.failed.failedJobs[0].jobId} />
              </div>
              <div className="flex flex-col space-y-1">
                {hostRev.failed?.failedJobs.map((j, i, jobs) => (
                  <div
                    key={i}
                    className="flex flex-col space-x-2 items-start border rounded-md p-1"
                  >
                    <code>
                      attempt {i + 1}/{jobs.length}
                    </code>
                    <div className="ml-4 flex flex-row items-center space-x-1">
                      <ClipboardCopy text={j.jobId}>
                        <code>{j.jobId}</code>
                      </ClipboardCopy>
                      <div className="flex flex-row items-center space-x-1">
                        <div>finished</div>
                        <DateTime date={j.finishedAt} />
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <InProgressDeployments
              revisions={props.revisions}
              inProgress={hostRev.inProgress}
              verb="deploying"
            />
          )}

          <LatestDeployment
            revisions={props.revisions}
            finished={hostRev.finished}
          />
        </div>
      </div>
    </div>
  );
}
