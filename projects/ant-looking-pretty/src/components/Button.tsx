"use client";

import { useRouter } from "next/navigation";
import { PropsWithChildren } from "react";

export const Button = (props: PropsWithChildren<{ path: string }>) => {
  const { push } = useRouter();
  return (
    <button className="cursor-pointer" onClick={() => push(props.path)}>
      {props.children}
    </button>
  );
};
