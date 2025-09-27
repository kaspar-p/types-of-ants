"use client";

import { Heart, Info } from "lucide-react";
import { ReleasedAnt } from "@/server/queries";
import { useEffect, useState } from "react";
import { useMediaQuery } from "@uidotdev/usehooks";
import { favorite, unfavorite } from "@/server/posts";
import { useUser } from "@/state/userContext";
import { useRouter } from "next/navigation";

export type AntTextProps = {
  ant: ReleasedAnt;
};

const ICON_SIZE = 14;

export function AntText(props: AntTextProps) {
  const displayIcon = true;

  const [ant, setAnt] = useState<ReleasedAnt | undefined>(undefined);

  useEffect(() => {
    setAnt(props.ant);
  }, []);

  const { push } = useRouter();

  const { user } = useUser();

  const [hover, setHover] = useState<boolean>(false);
  const [clicked, setClicked] = useState<boolean>(false);

  const canLike = user.weakAuth && user.loggedIn;
  const liked = canLike && !!ant?.favoritedAt;

  const cannotHover = useMediaQuery("only screen and (max-width : 768px)");

  if (!ant) return <div></div>;

  return (
    <div
      className={
        "flex flex-col w-full justify-center rounded-md cursor-pointer" +
        " " +
        ((cannotHover ? clicked : clicked || hover) ? " bg-slate-100" : "")
      }
      onMouseEnter={() => !cannotHover && setHover(true)}
      onMouseLeave={() => !cannotHover && setHover(false)}
      onMouseDown={(e) => {
        if ((e.target as unknown as { id: string }).id === "login-link") {
          push("/login");
          return;
        }
        setClicked(!clicked);
      }}
    >
      <div className="flex flex-row justify-between m-1">
        <div className="w-10/12">
          {clicked ? <strong>{ant.antName}</strong> : <div>{ant.antName}</div>}
        </div>
        <div className="px-1 self-center space-x-1">
          {displayIcon && (
            <Info
              size={ICON_SIZE}
              color={
                (cannotHover ? clicked : clicked || hover) ? "black" : "gray"
              }
            />
          )}
          {canLike && (
            <AntHeart
              liked={liked}
              enableHover={true}
              handleClick={async () => {
                if (liked) {
                  const res = await unfavorite({ antId: ant.antId });

                  switch (res.status) {
                    case 500: {
                      break;
                    }
                    case 400: {
                      console.error(await res.json());
                      break;
                    }
                    case 200: {
                      ant.favoritedAt = null;
                      setAnt({ ...ant, favoritedAt: null });
                      break;
                    }
                  }
                } else {
                  const res = await favorite({ antId: ant.antId });

                  switch (res.status) {
                    case 500: {
                      break;
                    }
                    case 400: {
                      console.error(await res.json());
                      break;
                    }
                    case 200: {
                      const body: { favoritedAt: string } = await res.json();
                      ant.favoritedAt = body.favoritedAt;
                      setAnt({ ...ant, favoritedAt: body.favoritedAt });
                      break;
                    }
                  }
                }
              }}
            />
          )}
        </div>
      </div>
      {clicked && (
        <div className="flex flex-col justify-between space-y-1 mb-2 pl-4">
          {
            <div>
              suggested by{" "}
              {ant.createdByUsername === "nobody"
                ? "an anonymous user"
                : `@${ant.createdByUsername}`}{" "}
              at {formatDatetime(new Date(ant.createdAt))}
            </div>
          }
          {
            <div>
              released at {formatDatetime(new Date(ant.release.createdAt))} in
              discovery #{ant.release.releaseNumber}
            </div>
          }
          {canLike ? (
            liked && ant.favoritedAt ? (
              <div className="flex flex-row space-x-1">
                {/* <AntHeart liked={liked} enableHover={false} /> */}
                favorited at {formatDatetime(new Date(ant.favoritedAt))}
              </div>
            ) : (
              <div>not your favorite {":("}</div>
            )
          ) : (
            <div>
              <a id="login-link" href="/login">
                login
              </a>{" "}
              for more!
            </div>
          )}
        </div>
      )}
    </div>
  );
}

const formatDatetime = (d: Date) => {
  return d.toLocaleTimeString() + " " + d.toLocaleDateString();
};

export type AntHeartProps = {
  liked: boolean;
  enableHover: boolean;
  handleClick?: () => void | Promise<void>;
};

function AntHeart({ liked, enableHover, handleClick }: AntHeartProps) {
  const [likeHover, setLikeHover] = useState<boolean>(false);

  return (
    <Heart
      className={
        "" +
        " " +
        (liked
          ? likeHover && enableHover
            ? "fill-red-800"
            : "fill-red-600"
          : "")
      }
      onMouseEnter={() => setLikeHover(true)}
      onMouseLeave={() => setLikeHover(false)}
      onMouseDown={(e) => {
        e.preventDefault();
        e.stopPropagation();
        handleClick?.();
      }}
      size={ICON_SIZE}
      color={
        liked
          ? likeHover && enableHover
            ? "#991b1b"
            : "red"
          : likeHover && enableHover
          ? "black"
          : "gray"
      }
    />
  );
}
