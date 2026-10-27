# AWS Lambda agent setup

Fetched from [docs.aws.amazon.com/lambda/…/aws-lambda-agent-setup.md](https://docs.aws.amazon.com/lambda/latest/dg/samples/aws-lambda-agent-setup.md). Kept here so the Marty alternatives tree documents how an agent should prepare for Lambda work.

## Validate authentication with AWS

```bash
aws sts get-caller-identity
```

If this fails (missing / expired credentials), use short-lived credentials via `aws login` (AWS CLI ≥ 2.32). Do not paste long-lived access keys unless you explicitly choose that fallback.

## Install the AWS serverless skill (Cursor and similar)

```bash
npx skills add https://github.com/aws/agent-toolkit-for-aws --skill aws-serverless --yes --global
```

### Optional specialized Lambda skills

```bash
npx skills add https://github.com/aws/agent-toolkit-for-aws/tree/main/skills/specialized-skills/serverless-skills/aws-lambda-microvms --yes --global
npx skills add https://github.com/aws/agent-toolkit-for-aws/tree/main/skills/specialized-skills/serverless-skills/aws-lambda-managed-instances --yes --global
npx skills add https://github.com/aws/agent-toolkit-for-aws/tree/main/skills/specialized-skills/serverless-skills/aws-lambda-durable-functions --yes --global
```

## Form factor for this tree

Per **aws-serverless**: Rust HTTP samples here are **Event Functions · On-Demand** (non-Node.js → Event Functions; no continuous-traffic signal → On-Demand). Public URL for `01_basic`: Function URL.
