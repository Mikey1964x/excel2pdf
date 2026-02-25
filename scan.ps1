# security scans

# go install golang.org/x/vuln/cmd/govulncheck@latest
govulncheck -show verbose ./... > vulncheck.log

# go install github.com/securego/gosec/v2/cmd/gosec@latest
gosec ./... > gosec.log

# go install honnef.co/go/tools/cmd/staticcheck@latest
staticcheck ./... > staticcheck.log

go vet ./... > govet.log