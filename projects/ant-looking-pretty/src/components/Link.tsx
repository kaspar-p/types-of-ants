"use client";

import { webAction } from "@/server/posts";
import NextLink from "next/link";
import { PropsWithChildren } from "react";

export const Link = (props: PropsWithChildren<{ href: string }>) => {
  return (
    <NextLink
      href={props.href}
      onClick={() =>
        webAction({
          action: "visit",
          targetType: "page",
          target: props.href,
        })
      }
    >
      {props.children}
    </NextLink>
  );
};
