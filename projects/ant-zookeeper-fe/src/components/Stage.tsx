import { HostGroup } from "./HostGroup";
import { color, Progress, revisions, type Stage } from "./Pipeline";
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
      <div className="flex flex-col border">
        <div className="border-b">
          <div
            className={`p-2 ${color(props.revisions, stageRev.finished?.revision).bg}`}
          >
            <div className="text-xl flex flex-row">
              {props.stage.stageName}
              <div className="ml-2 text-sm self-center">
                (<i>type: {props.stage.stageType.type}</i>)
              </div>
            </div>
          </div>
        </div>

        <div className="flex flex-col space-y-2 p-2">
          <div className="flex flex-row space-x-2">
            {stageRev.inProgress.length > 0 && (
              <span className="flex flex-row items-center">
                deploying {} in progress:{" "}
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

            <span className="flex flex-row items-center space-x-1">
              <div>latest: </div>
              {stageRev.finished ? (
                <RevisionBox
                  revs={props.revisions}
                  revision={stageRev.finished?.revision}
                />
              ) : (
                <span className="text-sm"> never</span>
              )}
            </span>
          </div>

          {props.stage.stageType.type === "deploy" ? (
            <HostGroup
              stage={props.stage as Stage & { stageType: { type: "deploy" } }}
              revisions={props.revisions}
              progress={props.progress}
            />
          ) : undefined}
        </div>
      </div>
    </div>
  );
}
