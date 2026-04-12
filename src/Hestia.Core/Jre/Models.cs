namespace Hestia.Core.Jre;

public enum JavaDistribution { Temurin, GraalVM, Zulu, Custom }

public sealed record JavaRuntime(
    string Id,
    int MajorVersion,
    string ExecutablePath,
    JavaDistribution Distribution,
    string? VendorString);

public sealed record JreInstallOptions(
    int MajorVersion,
    JavaDistribution Distribution = JavaDistribution.Temurin);
