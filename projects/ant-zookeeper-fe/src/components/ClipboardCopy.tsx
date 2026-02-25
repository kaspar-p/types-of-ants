"use client";

import { PropsWithChildren, useState } from "react";
import { Tooltip } from "react-tooltip";

export function ClipboardCopy(
  props: PropsWithChildren<{ text: string | undefined }>,
) {
  const [clickTimeout, setClickTimeout] = useState<NodeJS.Timeout | undefined>(
    undefined,
  );
  const [content, setContent] = useState<string>("copy?");

  const id = "copy-clipboard" + props.text;

  const handleClick = () => {
    setContent("copied!");

    if (clickTimeout) clearTimeout(clickTimeout);
    setClickTimeout(
      setTimeout(() => {
        setContent("copy?");
      }, 1000),
    );
  };

  return (
    <div
      className={props.text === undefined ? "" : "cursor-pointer"}
      onClick={() => {
        if (props.text) {
          navigator.clipboard.writeText(props.text);
          handleClick();
        }
      }}
    >
      <Tooltip
        hidden={props.text === undefined}
        afterHide={() => {
          setClickTimeout(undefined);
          setContent("copy?");
        }}
        clickable={true}
        closeEvents={{ mouseout: true, blur: true }}
        style={{ padding: 2 }}
        delayShow={10}
        delayHide={10}
        variant="light"
        opacity={1}
        place="top"
        border={"1px solid"}
        offset={2}
        arrowSize={3}
        id={id}
      />

      <div data-tooltip-id={id} data-tooltip-content={content}>
        {props.children}
      </div>
    </div>
  );
}
