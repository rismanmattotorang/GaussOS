{{- define "gaussos.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "gaussos.fullname" -}}
{{- printf "%s-%s" .Release.Name (include "gaussos.name" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "gaussos.labels" -}}
app.kubernetes.io/name: {{ include "gaussos.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version }}
{{- end -}}

{{- define "gaussos.selectorLabels" -}}
app.kubernetes.io/name: {{ include "gaussos.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}
