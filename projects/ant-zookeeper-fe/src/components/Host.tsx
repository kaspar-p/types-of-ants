import {
  color,
  formatDatetime,
  type Host,
  Progress,
  revisions,
} from "./Pipeline";
type HostProps = {
  host: Host;
  progress: Progress;
  revisions: string[];
};

export function Host(props: HostProps) {
  const hostRev = revisions(props.progress, props.host.name);

  return (
    <div>
      <div className="border">
        <div
          className={`p-2 border-b border-b-black flex flex-row ${color(props.revisions, hostRev.finished?.revision).bg}`}
        >
          <code>{props.host.name}</code>{" "}
          <div className="ml-2 text-sm self-center">({props.host.arch}) </div>
        </div>

        {hostRev.inProgress.length > 0 && (
          <div className="p-2 text-sm">
            deploying {hostRev.inProgress.length} in progress:{" "}
            {hostRev.inProgress
              .map((r) => `${r.revision} ${r.reachedAt}`)
              .join(", ")}
          </div>
        )}

        <div className="p-2 text-sm">
          deployed at:{" "}
          {hostRev.finished
            ? formatDatetime(hostRev.finished.reachedAt)
            : "never"}
        </div>
      </div>
    </div>
  );
}
