import { Host } from "./Host";
import { color, Progress, Stage } from "./Pipeline";
import { RevisionBox } from "./RevisionBox";

type HostGroupProps = {
  stage: Stage & { stageType: { type: "deploy" } };
  progress: Progress;
  revisions: string[];
};

export function HostGroup({ stage, revisions, progress }: HostGroupProps) {
  const hgProgress = progress[stage.stageType.hostGroup.hostGroupId];

  return (
    <div className="border">
      <div
        className={`p-2 border-b border-b-black flex flex-row ${color(revisions, hgProgress?.latestSuccessfulRevision?.revision).bg}`}
      >
        {stage.stageType.hostGroup.name}
        <div className="ml-2 text-sm self-center">
          (<i>environment: {stage.stageType.hostGroup.environment}</i>)
        </div>
      </div>
      <div className="flex flex-col space-y-2 p-2">
        <div className="flex flex-row space-x-2">
          {hgProgress?.latestStartedRevision &&
          hgProgress?.latestStartedRevision !==
            hgProgress?.latestStartedRevision ? (
            <span className="flex flex-row items-center">
              in progress:{" "}
              <RevisionBox
                revs={revisions}
                revision={hgProgress?.latestStartedRevision?.revision}
              />
            </span>
          ) : undefined}

          <span className="flex flex-row items-center">
            latest:{" "}
            <RevisionBox
              revs={revisions}
              revision={hgProgress?.latestSuccessfulRevision?.revision}
            />
          </span>
        </div>

        {stage.stageType.hostGroup.hosts.length > 0 ? (
          stage.stageType.hostGroup.hosts.map((host, i: number) => (
            <Host
              key={i}
              host={host}
              progress={progress}
              revisions={revisions}
            />
          ))
        ) : (
          <div>No hosts!</div>
        )}
      </div>
    </div>
  );
}
