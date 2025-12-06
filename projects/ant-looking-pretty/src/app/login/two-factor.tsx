"use client";

import { useTimedText } from "@/components/useTimedText";
import { addEmail, addPhoneNumber, verificationAttempt } from "@/server/posts";
import { getUser, getUserSchema } from "@/server/queries";
import { useUser } from "@/state/userContext";
import { useRouter } from "next/navigation";
import { FormEvent, useState } from "react";

export const TwoFactorVerificationBox = () => {
  const [option, setOption] = useState<"email" | "phone">("email");
  const [key, setKey] = useState<string>("");
  const [keyValidationMsg, setKeyValidationMsg] = useTimedText("");
  const [keySuccessMsg, setKeySuccessMsg] = useTimedText("");
  const [sent, setSent] = useState<boolean>(false);
  const [otp, setOtp] = useState<string>("");

  const { setUser } = useUser();
  const { push } = useRouter();

  async function handleSend(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const res =
      option === "phone"
        ? await addPhoneNumber({ phoneNumber: key, forceSend: true })
        : await addEmail({ email: key, forceSend: true });
    switch (res.status) {
      case 500: {
        const msg = await res.text();
        setKeyValidationMsg(msg);
        break;
      }
      case 401: {
        setKeyValidationMsg(
          `unverified ${option === "email" ? "email" : "phone number"}`
        );
        break;
      }
      case 400: {
        const errors: { errors: { field: string; msg: string }[] } =
          await res.json();
        setKeyValidationMsg(errors.errors[0].msg);
        break;
      }
      case 409: {
        setKeyValidationMsg("already taken!");
        break;
      }
      case 200: {
        console.log(res.status, await res.json());
        setKeySuccessMsg("sent!");
        setSent(true);
        break;
      }
      default: {
        console.log("unknown", res.status, await res.json());
        break;
      }
    }
  }

  async function handleVerificationAttempt(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const res = await verificationAttempt({
      method:
        option === "phone"
          ? { phone: { phoneNumber: key, otp } }
          : { email: { email: key, otp } },
    });

    switch (res.status) {
      case 500: {
        const msg = await res.text();
        setKeyValidationMsg(msg.toLocaleLowerCase());
        break;
      }
      case 200: {
        console.log(res.status);
        setKey("");

        const user = getUserSchema.transformer(
          getUserSchema.schema.parse(await (await getUser()).json())
        );
        setUser({ weakAuth: true, loggedIn: true, user: user.user });

        push("/");
        break;
      }
      default: {
        console.log("unknown", res.status, await res.json());
        break;
      }
    }
  }

  return (
    <div>
      <div className="mb-2">
        enter your preferred two-factor authentication message. you will receive
        an SMS text message or email with a one-time code.
      </div>
      <form autoComplete="off" onSubmit={(event) => handleSend(event)}>
        <div className="grid grid-cols-3 gap-0">
          <select
            name="method"
            defaultValue={"email"}
            onChange={(e) => setOption(e.target.value as "phone" | "email")}
          >
            <option>email</option>
            <option>phone</option>
          </select>

          <input
            className="m-1"
            type="text"
            name="key"
            autoComplete="off"
            placeholder={
              option === "email" ? "email@domain.com" : "+1 (000) 111-2222"
            }
            value={key}
            onChange={(e) => {
              setKey(e.target.value);
              setKeyValidationMsg("");
            }}
          />
          <span
            className={`flex flex-col justify-center m-1 ${
              keyValidationMsg ? "text-red-600" : "text-green-600"
            } content-center`}
          >
            {keyValidationMsg || keySuccessMsg || ""}
          </span>
        </div>

        <div className="flex flex-row w-8/12">
          <input
            type="submit"
            className="w-full m-1"
            value={!sent ? "send one-time code" : "resend"}
          />
        </div>
      </form>

      {sent && (
        <form
          autoComplete="off"
          onSubmit={(event) => handleVerificationAttempt(event)}
        >
          <div className="grid grid-cols-3 gap-0">
            <span className="flex flex-col justify-center">one-time code:</span>
            <input
              className="m-1"
              type="text"
              name="otp"
              autoComplete="off"
              placeholder=""
              value={otp}
              onChange={(e) => {
                setOtp(e.target.value);
              }}
            />
          </div>
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {keyValidationMsg}
          </span>

          <div className="flex flex-row w-8/12">
            <input type="submit" className="w-full m-1" value="submit" />
          </div>
        </form>
      )}
    </div>
  );
};
