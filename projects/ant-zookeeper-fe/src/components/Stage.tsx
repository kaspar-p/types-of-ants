import { BoxTitle } from "./BoxTitle";
import { HostGroup } from "./HostGroup";
import { InProgressDeployments } from "./InProgressDeployments";
import { LatestDeployment } from "./LatestDeployment";
import { Progress, revisions, type Stage } from "./Pipeline";
import { RevisionBox } from "./RevisionBox";

export type StageProps = {
  stage: Stage;
  progress: Progress;
  revisions: string[];
};

export function Stage(props: StageProps) {
  const stageRev = revisions(props.progress, props.stage.stageId);

  return (
    <div>
      <div className="border rounded-lg">
        <BoxTitle
          revisions={props.revisions}
          finished={stageRev.finished}
          inProgress={stageRev.inProgress}
        >
          <div className="text-xl">{props.stage.stageName}</div>
          <div className="text-sm self-center">
            (<i>type: {props.stage.stageType.type}</i>)
          </div>
        </BoxTitle>

        <div className="flex flex-col space-y-2 p-2">
          <div className="flex flex-col space-y-2">
            {stageRev.inProgress.length > 0 && (
              <span className="flex flex-row items-center space-x-1">
                <div>
                  {props.stage.stageType.type === "build"
                    ? "building"
                    : "deploying"}{" "}
                  in progress:
                </div>
                {stageRev.inProgress.map((rev, i) => (
                  <div key={i} className="flex flex-col space-y-2">
                    <RevisionBox
                      revs={props.revisions}
                      revision={rev.revision}
                    />
                  </div>
                ))}
              </span>
            )}

            <InProgressDeployments
              revisions={props.revisions}
              inProgress={stageRev.inProgress}
              verb={
                props.stage.stageType.type === "build" ? "building" : undefined
              }
            />

            <LatestDeployment
              revisions={props.revisions}
              finished={stageRev.finished}
            />
          </div>

          {props.stage.stageType.type === "deploy"
            ? props.stage.stageType.hostGroups.map((hostGroup, i) => (
                <div key={i}>
                  <HostGroup
                    stage={
                      props.stage as Stage & { stageType: { type: "deploy" } }
                    }
                    hostGroup={hostGroup}
                    revisions={props.revisions}
                    progress={props.progress}
                  />
                </div>
              ))
            : undefined}
        </div>
      </div>
    </div>
  );
}
