// Swift IntoString typeclass example.

protocol IntoString {
    func intoString() -> String
}

// NOTE: tuples are compound types in Swift 
// and cannot be extended.
extension Int: IntoString {
    func intoString() -> String {
        return String(self)
    }
}

extension Array: IntoString where Element: IntoString {
    func intoString() -> String {
        let strs = self.map { $0.intoString() }
        let joined = strs.joined(separator: ", ")
        return "[" + joined + "]"
    }
}

// ------------
// Client

// let iArr = [0, 1, 2]
// print(iArr.intoString()) // Output: "[0, 1, 2]"

let dArr = [0.0, 1.1, 2.2]
print(dArr.intoString()) // Double doesn't conform to IntoString
