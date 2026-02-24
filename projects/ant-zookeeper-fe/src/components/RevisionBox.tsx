import { color } from "./Pipeline";

export function RevisionBox({
  revs,
  revision,
}: {
  revs: string[];
  revision: string | undefined;
}) {
  return (
    <span
      className={`${color(revs, revision).bg} w-6 h-6 flex justify-center items-center`}
    >
      <div>{color(revs, revision).i}</div>
    </span>
  );
}
