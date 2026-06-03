{{/*
Nombre completo del recurso (release + chart).
*/}}
{{- define "rest-api-java.fullname" -}}
{{- printf "%s-%s" .Release.Name .Chart.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Etiquetas comunes aplicadas a todos los recursos.
*/}}
{{- define "rest-api-java.labels" -}}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" }}
{{- with .Values.labels }}
{{ toYaml . }}
{{- end }}
{{- end -}}

{{/*
Selector mínimo (sin variables que cambian en upgrades).
*/}}
{{- define "rest-api-java.selectorLabels" -}}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}
