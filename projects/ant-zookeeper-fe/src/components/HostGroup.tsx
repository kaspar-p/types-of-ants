import { BoxTitle } from "./BoxTitle";
import { Host } from "./Host";
import { InProgressDeployments } from "./InProgressDeployments";
import { LatestDeployment } from "./LatestDeployment";
import { Progress, revisions, Stage } from "./Pipeline";

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
    <div className="border rounded-lg">
      <BoxTitle
        revisions={props.revisions}
        finished={hgRev.finished}
        inProgress={hgRev.inProgress}
      >
        <div>{props.stage.stageType.hostGroup.name}</div>
        <div className="text-sm self-center">
          (<i>environment: {props.stage.stageType.hostGroup.environment}</i>)
        </div>
      </BoxTitle>

      <div className="flex flex-col space-y-2 p-2">
        <div className="flex flex-col space-y-2">
          <InProgressDeployments
            revisions={props.revisions}
            inProgress={hgRev.inProgress}
          />

          <LatestDeployment
            revisions={props.revisions}
            finished={hgRev.finished}
          />
        </div>

        {props.stage.stageType.hostGroup.hosts.length > 0 ? (
          props.stage.stageType.hostGroup.hosts.map(
            (host, i: number, hosts) => (
              <Host
                key={i}
                index={i + 1}
                total={hosts.length}
                host={host}
                progress={props.progress}
                revisions={props.revisions}
              />
            ),
          )
        ) : (
          <div>No hosts!</div>
        )}
      </div>
    </div>
  );
}
