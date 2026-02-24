import { color, formatDatetime, type Host, Progress } from "./Pipeline";
type HostProps = {
  host: Host;
  progress: Progress;
  revisions: string[];
};

export function Host({ host, progress, revisions }: HostProps) {
  const hostProgress = progress[host.name];

  return (
    <div>
      <div className="border">
        <div
          className={`p-2 border-b border-b-black flex flex-row ${color(revisions, hostProgress?.latestSuccessfulRevision?.revision).bg}`}
        >
          <code>{host.name}</code>{" "}
          <div className="ml-2 text-sm self-center">({host.arch}) </div>
        </div>

        <div className="p-2 text-sm">
          deployed at:{" "}
          {hostProgress?.latestSuccessfulRevision?.createdAt
            ? formatDatetime(hostProgress.latestSuccessfulRevision.createdAt!)
            : "never"}
        </div>
      </div>
    </div>
  );
}
