import { ReactNode } from "react";
import { Host } from "./Host";
import { RevisionBox } from "./RevisionBox";
import { HostGroup } from "./HostGroup";

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
  stageType: { type: "build" } | { type: "deploy"; hostGroup: HostGroup };
};

export type Progress = Record<
  string,
  {
    latestStartedRevision?: { createdAt: string; revision: string };
    latestSuccessfulRevision?: { createdAt: string; revision: string };
  }
>;

export type GetPipelineResponse = {
  pipelineId: string;
  project: string;
  stages: Stage[];

  progress: Progress;

  revisions: string[];
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
): { bg: string; i: string } => {
  if (!revision)
    return {
      bg: "text-black bg-gray-200",
      i: "n/a",
    }; // targets that have never been deployed to get no background

  const i = revisions.indexOf(revision);
  if (i === -1 || i >= COLORS.length) {
    return { bg: COLORS[COLORS.length - 1], i: i.toString() };
  } else {
    return { bg: COLORS[i], i: i.toString() };
  }
};

const pad = (s: string, padTo: number, padWith: string): string => {
  if (s.length >= padTo) return s;

  return (
    Array.from({ length: padTo - s.length })
      .map(() => padWith)
      .join("") + s
  );
};

export function formatDatetime(date: Date | string): string {
  const d = new Date(date);

  const time = `${pad(d.getHours().toString(), 2, "0")}:${pad(d.getMinutes().toString(), 2, "0")}:${pad(d.getSeconds().toString(), 2, "0")}`;
  const dateStr = `${d.getFullYear()}-${pad(d.getMonth().toString(), 2, "0")}-${pad(d.getDate().toString(), 2, "0")}`;

  return `${time} ${dateStr}`;
}

export function Pipeline({ res }: PipelineProps) {
  console.log(res.project, res.progress, res.revisions);

  return (
    <div className="p-3 border flex flex-col space-y-3">
      <h3>{res.project}</h3>

      <div>
        <div className="flex flex-row space-x-4 space-y-2 flex-wrap">
          {res.revisions.map((revision, i, revs) => (
            <div key={revision}>
              <div className="flex flex-row border p-1">
                <span className="self-center">
                  revision {i}{" "}
                  <span className="text-xs self-center">({revision})</span>:
                </span>
                <RevisionBox revs={revs} revision={revision} />
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="flex flex-row space-x-6">
        {res.stages.map((stage, i: number) => (
          <div key={i}>
            <div className="flex flex-col border">
              <div id={`stage-title-${i}`} className="border-b">
                <div
                  className={`p-2 ${color(res.revisions, res.progress[stage.stageId]?.latestSuccessfulRevision?.revision).bg}`}
                >
                  <div className="text-xl flex flex-row">
                    {stage.stageName}
                    <div className="ml-2 text-sm self-center">
                      (<i>type: {stage.stageType.type}</i>)
                    </div>
                  </div>
                </div>
              </div>

              <div className="flex flex-col space-y-2 p-2">
                <div className="flex flex-row space-x-2">
                  {res.progress[stage.stageId]?.latestStartedRevision &&
                  res.progress[stage.stageId]?.latestStartedRevision
                    ?.revision !==
                    res.progress[stage.stageId]?.latestSuccessfulRevision
                      ?.revision ? (
                    <span className="flex flex-row items-center">
                      in progress:{" "}
                      <RevisionBox
                        revs={res.revisions}
                        revision={
                          res.progress[stage.stageId]?.latestStartedRevision
                            ?.revision
                        }
                      />
                    </span>
                  ) : null}

                  <span className="flex flex-row items-center space-x-1">
                    <div>latest: </div>
                    {res.revisions.length > 0 ? (
                      <RevisionBox
                        revs={res.revisions}
                        revision={
                          res.progress[stage.stageId]?.latestSuccessfulRevision
                            ?.revision
                        }
                      />
                    ) : (
                      <span className="text-sm"> never</span>
                    )}
                  </span>
                </div>

                {stage.stageType.type === "deploy" ? (
                  <HostGroup
                    stage={stage as Stage & { stageType: { type: "deploy" } }}
                    revisions={res.revisions}
                    progress={res.progress}
                  />
                ) : undefined}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
