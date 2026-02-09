# Open S3 Bucket

- **Severity:** Critical
- **Asset:** Nexus Portal
- **Date:** 2025/08/15
- **Location:** https://s3.amazonaws.com/nexus-uploads
- **Status:** Resolved

## Description

The `nexus-uploads` S3 bucket has a public ACL granting `s3:GetObject`
and `s3:PutObject` to `AllUsers`. Any unauthenticated user can list,
download, and upload files to the bucket.

The bucket contains user-uploaded documents including passport scans
and signed contracts.

**Impact:** Mass data exposure of sensitive PII and the ability for
attackers to serve malicious files from a trusted domain.
