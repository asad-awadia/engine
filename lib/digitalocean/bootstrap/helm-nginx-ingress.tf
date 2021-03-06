resource "helm_release" "nginx_ingress" {
  name = "nginx-ingress"
  chart = "common/charts/nginx-ingress"
  namespace = "nginx-ingress"
  create_namespace = true
  atomic = true
  max_history = 50

  # Because of NLB, svc can take some time to start
  timeout = 300
  values = [file("chart_values/nginx-ingress.yaml")]

  # Controller resources
  set {
    name = "controller.resources.limits.cpu"
    value = "200m"
  }

  set {
    name = "controller.resources.requests.cpu"
    value = "100m"
  }

  set {
    name = "controller.resources.limits.memory"
    value = "768Mi"
  }

  set {
    name = "controller.resources.requests.memory"
    value = "768Mi"
  }

  # Default backend resources
  set {
    name = "defaultBackend.resources.limits.cpu"
    value = "20m"
  }

  set {
    name = "defaultBackend.resources.requests.cpu"
    value = "10m"
  }

  set {
    name = "defaultBackend.resources.limits.memory"
    value = "32Mi"
  }

  set {
    name = "defaultBackend.resources.requests.memory"
    value = "32Mi"
  }

  set {
    name = "forced_upgrade"
    value = var.forced_upgrade
  }

  depends_on = [
    digitalocean_kubernetes_cluster.kubernetes_cluster,
  ]
}