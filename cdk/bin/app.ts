#!/usr/bin/env node
import { App } from "aws-cdk-lib";
import { WxveChatStack } from "../lib/wxve-chat-stack";

const app = new App();

const domainName = app.node.tryGetContext("domainName") || "chat.wxve.io";
const hostedZoneName = app.node.tryGetContext("hostedZoneName") || "wxve.io";

new WxveChatStack(app, "WxveChatStack", {
  domainName,
  hostedZoneName,
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: "us-east-1",
  },
});
