import Link from "next/link";
import { PropsWithChildren } from "react";

export type BlogPageProps = {
  title: string;
  formattedDate: string;
  href: string;
};

export function BlogPage(props: PropsWithChildren<BlogPageProps>) {
  return (
    <div className="flex flex-row justify-evenly">
      <div></div>
      <div className="w-2xl">
        <div className="text-2xl">
          <strong>{props.title}</strong>
        </div>
        <div>posted {props.formattedDate}</div>
        <Link href={"/blog"}>back to /blog</Link>

        <div className="pt-4">{props.children}</div>
      </div>
      <div></div>
    </div>
  );
}
