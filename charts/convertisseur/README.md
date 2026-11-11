# convertisseur

![Version: 0.1.0](https://img.shields.io/badge/Version-0.1.0-informational?style=flat-square) ![Type: application](https://img.shields.io/badge/Type-application-informational?style=flat-square) ![AppVersion: 0.1.0](https://img.shields.io/badge/AppVersion-0.1.0-informational?style=flat-square)

Convertisseur, a gamified web app for learning the basics of Islam and Arabic, served as one image by a Rust server.

**Homepage:** <https://github.com/therealm-tech/convertisseur>

## Maintainers

| Name | Email | Url |
| ---- | ------ | --- |
| Guillaume Leroy | <gleroy@therealm.tech> |  |

## Source Code

* <https://github.com/therealm-tech/convertisseur>

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| convertisseur.affinity | object | `{}` | Affinity of the pods. |
| convertisseur.caCerts.existing.key | string | `"ca-certificates.crt"` | Key in the resource holding the PEM bundle, also the mounted file's name. |
| convertisseur.caCerts.existing.kind | string | `"ConfigMap"` | Kind of the resource holding the PEM bundle: `ConfigMap` or `Secret`. |
| convertisseur.caCerts.existing.name | string | `nil` | Name of a ConfigMap or Secret holding a PEM bundle to mount. Set → the bundle is mounted and `SSL_CERT_FILE` points at it. The object store's TLS client trusts that bundle instead of the image's roots, so it must hold every root the connection needs. |
| convertisseur.clientIpHeader | string | `nil` | Header in which the reverse proxy in front of the server passes the client's IP, e.g. `X-Forwarded-For`. Leave unset when clients connect directly, since they can forge it. Behind an ingress, set it, or every client shares the proxy's rate limits. |
| convertisseur.cookieSecure | bool | `true` | Mark the session cookie `Secure`, so that browsers only send it over HTTPS. Turn it off only to serve plain HTTP. |
| convertisseur.database.existingSecret.key | string | `"DATABASE_URL"` | Key in that Secret holding the connection string. |
| convertisseur.database.existingSecret.name | string | `nil` | Name of the Secret holding the PostgreSQL connection string, passed as `DATABASE_URL`. Required. |
| convertisseur.database.maxConnections | int | `10` | Size of the connection pool of each pod, passed as `DB_MAX_CONNECTIONS`. |
| convertisseur.extraEnv | list | `[]` | Extra environment variables for the container. |
| convertisseur.extraVolumeMounts | list | `[]` | Extra volume mounts for the container. |
| convertisseur.extraVolumes | list | `[]` | Extra volumes for the pod. |
| convertisseur.fullnameOverride | string | `nil` | Override of the full resource name. |
| convertisseur.image.pullPolicy | string | `"IfNotPresent"` | Image pull policy. |
| convertisseur.image.repository | string | `"ghcr.io/therealm-tech/convertisseur"` | Image repository. |
| convertisseur.image.tag | string | `nil` | Image tag. Defaults to `.Chart.AppVersion`. |
| convertisseur.imagePullSecrets | list | `[]` | Overrides `global.imagePullSecrets` for this component when not empty. |
| convertisseur.ingress.annotations | object | `{}` | Annotations added to the Ingress. |
| convertisseur.ingress.className | string | `nil` | Ingress class. |
| convertisseur.ingress.enabled | bool | `false` | Create an Ingress. The session cookie is `Secure`, so the app must be served over HTTPS. |
| convertisseur.ingress.host | string | `nil` | Host name served. Required when the Ingress is enabled. |
| convertisseur.ingress.tls.secretName | string | `nil` | Name of the Secret holding the host's certificate. Unset → the Ingress declares no TLS section. |
| convertisseur.livenessProbe | object | `{"failureThreshold":3,"httpGet":{"path":"/healthz","port":"http"},"periodSeconds":10}` | Liveness probe: the process serves HTTP. |
| convertisseur.log.filter | string | `"info"` | [`tracing` filter directive](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives), passed as `LOG_FILTER`. |
| convertisseur.log.format | string | `"text"` | Log format, `text` or `json`, passed as `LOG_FORMAT`. |
| convertisseur.nameOverride | string | `nil` | Override of the chart name in resource names. |
| convertisseur.nodeSelector | object | `{}` | Node selector of the pods. |
| convertisseur.podAnnotations | object | `{}` | Annotations added to the pods. |
| convertisseur.podDisruptionBudget.enabled | bool | `false` | Create a PodDisruptionBudget. With a single replica it blocks node drains, so enable it with `replicaCount` of 2 or more. |
| convertisseur.podDisruptionBudget.minAvailable | int | `1` | Pods that must stay available during a voluntary disruption. |
| convertisseur.podLabels | object | `{}` | Labels added to the pods. |
| convertisseur.podSecurityContext | object | `{"fsGroup":65532,"runAsGroup":65532,"runAsNonRoot":true,"runAsUser":65532,"seccompProfile":{"type":"RuntimeDefault"}}` | Pod-level security context; loosening it breaks the restricted Pod Security Standard. The IDs match the image's `nonroot` user. |
| convertisseur.podSecurityContext.fsGroup | int | `65532` | GID that owns the pod's mounted volumes. |
| convertisseur.podSecurityContext.runAsGroup | int | `65532` | GID of the container's process. |
| convertisseur.podSecurityContext.runAsNonRoot | bool | `true` | Refuse to start a container whose image runs as UID 0. |
| convertisseur.podSecurityContext.runAsUser | int | `65532` | UID of the container's process. |
| convertisseur.podSecurityContext.seccompProfile.type | string | `"RuntimeDefault"` | Seccomp profile of the pod. |
| convertisseur.port | int | `3000` | Port the server listens on, passed to it as `BIND_ADDR`. |
| convertisseur.readinessProbe | object | `{"failureThreshold":3,"httpGet":{"path":"/readyz","port":"http"},"periodSeconds":10}` | Readiness probe: PostgreSQL and the object store answer. |
| convertisseur.replicaCount | int | `1` | Number of pods. The login and registration rate limits are counted in each pod's memory, so every replica adds its own allowance per client IP. |
| convertisseur.resources | object | `{"limits":{"ephemeral-storage":"128Mi","memory":"256Mi"},"requests":{"cpu":"50m","ephemeral-storage":"64Mi","memory":"128Mi"}}` | Resources of the container. Uploads are buffered in memory and password hashing is memory-hard, hence the memory limit. |
| convertisseur.resources.limits.ephemeral-storage | string | `"128Mi"` | Ephemeral storage above which the pod is evicted. |
| convertisseur.resources.limits.memory | string | `"256Mi"` | Memory above which the container is killed. |
| convertisseur.resources.requests.cpu | string | `"50m"` | CPU the scheduler reserves. |
| convertisseur.resources.requests.ephemeral-storage | string | `"64Mi"` | Ephemeral storage the scheduler reserves. |
| convertisseur.resources.requests.memory | string | `"128Mi"` | Memory the scheduler reserves. |
| convertisseur.revisionHistoryLimit | int | `0` | Number of old revisions the workload keeps for rollback. |
| convertisseur.s3.accessKeyId.existingSecret.key | string | `"S3_ACCESS_KEY_ID"` | Key in that Secret holding the access key ID. |
| convertisseur.s3.accessKeyId.existingSecret.name | string | `nil` | Name of the Secret holding the access key ID, passed as `S3_ACCESS_KEY_ID`. Required. |
| convertisseur.s3.bucket | string | `nil` | Bucket holding the uploaded media, passed as `S3_BUCKET`. Required. Created at start when it does not exist. |
| convertisseur.s3.endpoint | string | `nil` | URL of the S3-compatible object store, passed as `S3_ENDPOINT`. Required. |
| convertisseur.s3.forcePathStyle | bool | `false` | Name the bucket in the URL's path rather than in its host name, passed as `S3_FORCE_PATH_STYLE`. Most self-hosted stores need `true`. |
| convertisseur.s3.region | string | `"us-east-1"` | Region of the bucket, passed as `S3_REGION`. |
| convertisseur.s3.secretAccessKey.existingSecret.key | string | `"S3_SECRET_ACCESS_KEY"` | Key in that Secret holding the secret access key. |
| convertisseur.s3.secretAccessKey.existingSecret.name | string | `nil` | Name of the Secret holding the secret access key, passed as `S3_SECRET_ACCESS_KEY`. Required. |
| convertisseur.securityContext | object | `{"allowPrivilegeEscalation":false,"capabilities":{"drop":["ALL"]},"readOnlyRootFilesystem":true}` | Container-level security context. The server writes nothing to its filesystem, so the root filesystem is read-only. |
| convertisseur.securityContext.allowPrivilegeEscalation | bool | `false` | Forbid a process from gaining more privileges than its parent. |
| convertisseur.securityContext.capabilities.drop | list | `["ALL"]` | Linux capabilities dropped from the container. |
| convertisseur.securityContext.readOnlyRootFilesystem | bool | `true` | Mount the container's root filesystem read-only. |
| convertisseur.service.port | int | `80` | Port the Service exposes. |
| convertisseur.service.type | string | `"ClusterIP"` | Service type. |
| convertisseur.serviceAccount.annotations | object | `{}` | Annotations added to the ServiceAccount. |
| convertisseur.serviceAccount.create | bool | `true` | Create a ServiceAccount for the pods. |
| convertisseur.serviceAccount.name | string | `nil` | Name of the ServiceAccount. Defaults to the full resource name. |
| convertisseur.startupProbe | object | `{"failureThreshold":30,"httpGet":{"path":"/healthz","port":"http"},"periodSeconds":5}` | Startup probe. The server applies the migrations and sets up the bucket before it listens, so a slow start must not trip the liveness probe. |
| convertisseur.superAdmin.password.existingSecret.key | string | `"SUPER_ADMIN_PASSWORD"` | Key in that Secret holding the password. |
| convertisseur.superAdmin.password.existingSecret.name | string | `nil` | Name of the Secret holding the super admin's password, at least 12 characters, passed as `SUPER_ADMIN_PASSWORD`. Required. Applied at every start, so a change needs a pod restart. |
| convertisseur.superAdmin.username | string | `nil` | Username of the super admin, passed as `SUPER_ADMIN_USERNAME`. Required. Changing it demotes the previous super admin to `admin`. |
| convertisseur.tolerations | list | `[]` | Tolerations of the pods. |
| convertisseur.topologySpreadConstraints | list | `[]` | Topology spread constraints of the pods. |
| global.imagePullSecrets | list | `[]` | Secrets, in the release namespace, that hold the credentials to pull the image. |

----------------------------------------------
Autogenerated from chart metadata using [helm-docs v1.14.2](https://github.com/norwoodj/helm-docs/releases/v1.14.2)
