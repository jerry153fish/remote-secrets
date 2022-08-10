use kube::CustomResourceExt;
fn main() {
    print!("{}", serde_yaml::to_string(&crd::RSecret::crd()).unwrap())
}
