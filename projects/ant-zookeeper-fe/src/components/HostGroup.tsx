import { Host } from "./Host";
import { color, Progress, revisions, Stage } from "./Pipeline";
import { RevisionBox } from "./RevisionBox";

type HostGroupProps = {
  stage: Stage & { stageType: { type: "deploy" } };
  progress: Progress;
  revisions: string[];
};

export function HostGroup(props: HostGroupProps) {
  const hgRev = revisions(
    props.progress,
    props.stage.stageType.hostGroup.hostGroupId,
  );

  return (
    <div className="border">
      <div
        className={`p-2 border-b border-b-black flex flex-row ${color(props.revisions, hgRev.finished?.revision).bg}`}
      >
        {props.stage.stageType.hostGroup.name}
        <div className="ml-2 text-sm self-center">
          (<i>environment: {props.stage.stageType.hostGroup.environment}</i>)
        </div>
      </div>
      <div className="flex flex-col space-y-2 p-2">
        <div className="flex flex-row space-x-2">
          {hgRev.inProgress.length > 0 ? (
            <span className="flex flex-row items-center">
              in progress:{" "}
              <RevisionBox
                revs={props.revisions}
                revision={hgRev.inProgress[0].revision}
              />
            </span>
          ) : undefined}

          <span className="flex flex-row items-center">
            latest:{" "}
            <RevisionBox
              revs={props.revisions}
              revision={hgRev.finished?.revision}
            />
          </span>
        </div>

        {props.stage.stageType.hostGroup.hosts.length > 0 ? (
          props.stage.stageType.hostGroup.hosts.map((host, i: number) => (
            <Host
              key={i}
              host={host}
              progress={props.progress}
              revisions={props.revisions}
            />
          ))
        ) : (
          <div>No hosts!</div>
        )}
      </div>
    </div>
  );
}
