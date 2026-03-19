import { Host } from "./Host";
import { RevisionBox } from "./RevisionBox";
import { HostGroup } from "./HostGroup";
import { Stage } from "./Stage";
import { ClipboardCopy } from "./ClipboardCopy";

export type Host = {
  name: string;
  arch: string;
};

export type HostGroup = {
  hostGroupId: string;
  name: string;
  environment: string;
  hosts: Host[];
};

export type Stage = {
  stageId: string;
  stageName: string;
  stageType:
    | { type: "build" }
    | {
        type: "deploy";
        hostGroups: HostGroup[];
      };
};

export type Revision = {
  revision: string;
  reachedAt: string;
};

export type FailedJob = {
  jobId: string;
  startedAt: string;
  finishedAt: string;
};

export type FailedRevision = {
  revision: string;
  failedJobs: FailedJob[];
};

export type Progress = Record<
  string,
  {
    startedRevisions?: Revision[];
    finishedRevisions?: Revision[];
    failedRevisions?: FailedRevision[];
  }
>;

export type GetPipelineResponse = {
  pipelineId: string;
  name: string;

  stages?: Stage[][];

  progress: Progress;

  revisions?: string[];
};

export type PipelineProps = {
  res: GetPipelineResponse;
};

const COLORS = [
  "text-white bg-green-700",
  "text-black bg-green-500",
  "text-black bg-green-300",
  "text-white bg-blue-800",
  "text-black bg-blue-500",
  "text-black bg-blue-300",
  "text-black bg-fuchsia-700",
  "text-black bg-fuchsia-500",
  "text-black bg-fuchsia-300",
  "text-white bg-black",
];
export const color = (
  revisions: string[],
  revision: string | undefined,
): { text: string; bg: string; i: string } => {
  if (!revision)
    return {
      text: "text-black",
      bg: "bg-gray-200",
      i: "n/a",
    }; // targets that have never been deployed to get no background

  let i = revisions.indexOf(revision);
  if (i === -1 || i >= COLORS.length) {
    i = COLORS.length - 1;
  }

  const [text, bg] = COLORS[i].split(" ");
  return { text, bg, i: i.toString() };
};

export const revisions = (
  progress: Progress,
  id: string,
): {
  inProgress: Revision[];
  failed: FailedRevision | undefined;
  finished: Revision | undefined;
} => {
  const prog = progress[id];
  if (!prog) {
    return { inProgress: [], finished: undefined, failed: undefined };
  }

  const inProgress: Revision[] = (prog.startedRevisions ?? []).filter(
    (r1) =>
      !(prog?.finishedRevisions ?? []).find(
        (r2) => r1.revision === r2.revision,
      ),
  );
  // .filter(
  //   (r1) =>
  //     !(prog?.failedRevisions ?? []).find(
  //       (r2) => r1.revision === r2.revision,
  //     ),
  // );

  const finished: Revision | undefined = prog?.finishedRevisions?.[0];

  const failed = prog.failedRevisions;

  return { inProgress, finished, failed: failed?.[0] };
};

export function Pipeline({ res }: PipelineProps) {
  console.log(res.name, res.progress, res.revisions);

  return (
    <div className="p-3 border rounded-2xl flex flex-col space-y-3 w-fit">
      <h3 className="flex flex-row space-x-2">
        <div>{res.name}</div>
        <ClipboardCopy text={res.pipelineId}>
          (<code className="text-sm">{res.pipelineId}</code>)
        </ClipboardCopy>
      </h3>

      <div>
        <div className="flex flex-row space-x-4 space-y-2 flex-wrap">
          {res.revisions?.map((revision, i, revs) => (
            <div key={revision}>
              <ClipboardCopy text={revision}>
                <div className="flex flex-row border rounded-md p-1 space-x-1">
                  <span className="self-center">
                    <code>{revision}</code>
                  </span>
                  <RevisionBox revs={revs} revision={revision} />
                </div>
              </ClipboardCopy>
            </div>
          ))}
        </div>
      </div>

      <div className="flex flex-row space-x-6">
        {res.stages?.map((phase, i: number) => (
          <div key={i}>
            <div className="flex flex-col space-y-2">
              {phase.map((stage, i) => (
                <div key={i}>
                  <Stage
                    revisions={res.revisions ?? []}
                    progress={res.progress}
                    stage={stage}
                  />
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
