{{- define "convertisseur.name" -}}
{{- default .Chart.Name .Values.convertisseur.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "convertisseur.fullname" -}}
{{- if .Values.convertisseur.fullnameOverride -}}
{{- .Values.convertisseur.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.convertisseur.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{- define "convertisseur.selectorLabels" -}}
app.kubernetes.io/name: {{ include "convertisseur.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{- define "convertisseur.labels" -}}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{ include "convertisseur.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{- define "convertisseur.serviceAccountName" -}}
{{- $sa := .Values.convertisseur.serviceAccount -}}
{{- if $sa.create -}}
{{- default (include "convertisseur.fullname" .) $sa.name -}}
{{- else -}}
{{- default "default" $sa.name -}}
{{- end -}}
{{- end -}}

{{- define "convertisseur.image" -}}
{{- printf "%s:%s" .Values.convertisseur.image.repository (default .Chart.AppVersion .Values.convertisseur.image.tag) -}}
{{- end -}}

{{/* Env var read from a pre-existing Secret: dict "env" "NAME" "ref" <{name, key}> "path" "values.path". */}}
{{- define "convertisseur.secretEnv" -}}
- name: {{ .env }}
  valueFrom:
    secretKeyRef:
      name: {{ required (printf "%s.name is required: the chart reads this credential from an existing Secret" .path) .ref.name }}
      key: {{ .ref.key }}
{{- end -}}

{{- define "convertisseur.caCertsPath" -}}
/etc/convertisseur/ca/{{ .Values.convertisseur.caCerts.existing.key }}
{{- end -}}

{{- define "convertisseur.caCertsValidate" -}}
{{- if not (has .Values.convertisseur.caCerts.existing.kind (list "ConfigMap" "Secret")) -}}
{{- fail "convertisseur.caCerts.existing.kind must be ConfigMap or Secret" -}}
{{- end -}}
{{- end -}}
