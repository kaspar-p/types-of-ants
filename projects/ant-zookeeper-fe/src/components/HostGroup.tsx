import { BoxTitle } from "./BoxTitle";
import { Host } from "./Host";
import { InProgressDeployments } from "./InProgressDeployments";
import { LatestDeployment } from "./LatestDeployment";
import { type HostGroup, Progress, revisions, Stage } from "./Pipeline";

type HostGroupProps = {
  stage: Stage & { stageType: { type: "deploy" } };
  hostGroup: HostGroup;
  progress: Progress;
  revisions: string[];
};

export function HostGroup(props: HostGroupProps) {
  const hgRev = revisions(props.progress, props.hostGroup.hostGroupId);

  return (
    <div className="border rounded-lg">
      <BoxTitle
        revisions={props.revisions}
        finished={hgRev.finished}
        inProgress={hgRev.inProgress}
      >
        <div>{props.hostGroup.name}</div>
        <div className="text-sm self-center">
          (<i>environment: {props.hostGroup.environment}</i>)
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

        {props.hostGroup.hosts.length > 0 ? (
          props.hostGroup.hosts.map((host, i: number, hosts) => (
            <Host
              key={i}
              index={i + 1}
              total={hosts.length}
              hostGroup={props.hostGroup}
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
