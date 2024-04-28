import dotenv from "dotenv";
import { readFile } from "fs-extra";
import path from "path";

import nodemailer, { SentMessageInfo } from "nodemailer";

const log = {
  info: (...args: any[]): void => {
    return console.info("[ant-using-email]", "[inf]", ...args);
  },
  error: (...args: any[]): void => {
    return console.error("[ant-using-email]", "[err]", ...args);
  },
};

dotenv.config({ path: ".env.email" });
async function sendEmail(
  transporter: nodemailer.Transporter,
  to: string,
  content: string
): Promise<SentMessageInfo | Error> {
  const mailDetails = {
    from: process.env.GMAIL_USER,
    to,
    subject: "happy new year",
    text: content,
  };

  await transporter
    .sendMail(mailDetails)
    .catch((err) => log.error(`Failed to send to ${to}! Error:`, err));
}

async function getEmailContent(): Promise<string> {
  return await readFile(
    path.join(__dirname, "..", "emails", "12-31-2023", "new_year.txt"),
    {
      encoding: "utf8",
    }
  );
}

async function getEmails(): Promise<string[]> {
  const emailsFileContent = await readFile(
    path.join(__dirname, "..", "data", "emails.csv"),
    {
      encoding: "utf8",
    }
  );

  const emails = emailsFileContent
    .split("\n")
    .map((s) => s.trim().split(",")[0].replace(/"/g, ""))
    .slice(1)
    .filter((s) => s !== "");

  return emails;
}

async function main() {
  log.info("Starting...");

  const mailTransporter = nodemailer.createTransport({
    service: "gmail",
    auth: {
      user: process.env.GMAIL_USER,
      pass: process.env.GMAIL_PASSWORD,
    },
  });

  const content = await getEmailContent();
  const emails = await getEmails();
  log.info("Got emails: ", emails);

  for (const email of emails) {
    log.info("Sending to", email);
    await sendEmail(mailTransporter, email, content);
  }

  log.info("✅ Finished!");
}

main().catch((e) => {
  log.error("❌ Error occurred!");
  throw e;
});
