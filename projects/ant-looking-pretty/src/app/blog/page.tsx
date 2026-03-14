import Link from "next/link";
import { formatDate } from "../feed/page";

export default function Blog() {
  return (
    <div className="flex flex-row justify-evenly">
      <div></div>
      <div className="w-2xl text-left">
        <div className="text-2xl">
          <strong>ants on a blog</strong>
        </div>
        <BlogListEntry
          title="typesofants.org moves to nyc"
          href="/blog/2026/03/12/typesofants.org2nyc"
          formattedDate={"Feb 28 2026 - Mar 12 2026"}
        />
      </div>
      <div></div>
    </div>
  );
}

type BlogListEntryProps = {
  title: string;
  href: string;
  formattedDate: string;
};

function BlogListEntry(props: BlogListEntryProps) {
  return (
    <div>
      <div className="pl-6 flex flex-row space-x-3">
        <Link href={props.href}>
          <div className="text-xl">{props.title}</div>
        </Link>
        <div className="place-self-center">{props.formattedDate}</div>
      </div>
    </div>
  );
}
