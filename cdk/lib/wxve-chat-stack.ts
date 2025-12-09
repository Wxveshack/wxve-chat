import { Stack, StackProps, RemovalPolicy, CfnOutput } from "aws-cdk-lib";
import { Bucket, BlockPublicAccess } from "aws-cdk-lib/aws-s3";
import {
  Distribution,
  ViewerProtocolPolicy,
  CachePolicy,
} from "aws-cdk-lib/aws-cloudfront";
import { S3BucketOrigin } from "aws-cdk-lib/aws-cloudfront-origins";
import { Certificate, CertificateValidation } from "aws-cdk-lib/aws-certificatemanager";
import { HostedZone, ARecord, RecordTarget } from "aws-cdk-lib/aws-route53";
import { CloudFrontTarget } from "aws-cdk-lib/aws-route53-targets";
import { BucketDeployment, Source } from "aws-cdk-lib/aws-s3-deployment";
import { Construct } from "constructs";
import { join } from "path";

interface WxveChatStackProps extends StackProps {
  domainName: string;
  hostedZoneName: string;
}

export class WxveChatStack extends Stack {
  constructor(scope: Construct, id: string, props: WxveChatStackProps) {
    super(scope, id, props);

    const { domainName, hostedZoneName } = props;

    // S3 bucket for static files
    const bucket = new Bucket(this, "WebsiteBucket", {
      removalPolicy: RemovalPolicy.DESTROY,
      autoDeleteObjects: true,
      blockPublicAccess: BlockPublicAccess.BLOCK_ALL,
    });

    // Look up hosted zone
    const hostedZone = HostedZone.fromLookup(this, "HostedZone", {
      domainName: hostedZoneName,
    });

    // SSL certificate
    const certificate = new Certificate(this, "Certificate", {
      domainName,
      validation: CertificateValidation.fromDns(hostedZone),
    });

    // CloudFront distribution
    const distribution = new Distribution(this, "Distribution", {
      defaultBehavior: {
        origin: S3BucketOrigin.withOriginAccessControl(bucket),
        viewerProtocolPolicy: ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        cachePolicy: CachePolicy.CACHING_OPTIMIZED,
      },
      domainNames: [domainName],
      certificate,
      defaultRootObject: "index.html",
      errorResponses: [
        {
          httpStatus: 404,
          responseHttpStatus: 200,
          responsePagePath: "/index.html",
        },
      ],
    });

    // DNS record
    new ARecord(this, "AliasRecord", {
      zone: hostedZone,
      recordName: domainName,
      target: RecordTarget.fromAlias(new CloudFrontTarget(distribution)),
    });

    // Deploy static files from dist/
    new BucketDeployment(this, "DeployWebsite", {
      sources: [Source.asset(join(__dirname, "../../dist"))],
      destinationBucket: bucket,
      distribution,
      distributionPaths: ["/*"],
    });

    // Outputs
    new CfnOutput(this, "DistributionDomainName", {
      value: distribution.distributionDomainName,
    });

    new CfnOutput(this, "WebsiteUrl", {
      value: `https://${domainName}`,
    });
  }
}
