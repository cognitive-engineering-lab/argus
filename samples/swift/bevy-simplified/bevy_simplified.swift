// Bevy simplified example

// TODO: this still doesn't work, but one of the problems
// with the bevy example is the implicitness of the Fn 
// conversion which Swift doesn't allow. I need to figure 
// out how to handle that in the same way.

// import Foundation

// protocol Resource {}
 
// struct Res<T> { }
 
struct TheWorld {}

protocol SystemParam {}

protocol System { }
 
protocol IntoSystemConfig { }
 
protocol IntoSystem { }
 
protocol ExclusiveSystemParam {}
 
struct IsExclusiveFunctionSystem {}
 
protocol ExclusiveSystemParamFunction {
//     associatedtype Marker
//     associatedtype In
//     associatedtype Out
//     associatedtype Param: ExclusiveSystemParam
}
 
struct ExclusiveFunctionSystem<Marker, F>: IntoSystem 
    where F: ExclusiveSystemParamFunction<Marker> { }
 
// struct FunctionSystem<Marker, F>: IntoSystem 
//     where F: SystemParamFunction<Marker> { }
// 
// struct App {
//     static func new() -> Self {
//         return App()
//     }
// 
//     func insertResource<T>(_ r: T) -> Self {
//         return self
//     }
// 
//     func addSystem<M>(_: M) -> Self where M: IntoSystemConfig {
//         return self
//     }
// 
//     func run() { }
// }
// 
// --------------------
// Extensions
 
// extension SystemParamFunction where Self: Func, F0: SystemParam, Func: (_ F0) -> Out {
//     typealias In = Void
//     typealias Out = Out
//     typealias Param = (F0,)
// }
 
extension ExclusiveSystemParamFunction where Self: Func, Func: (_ TheWorld) -> Out {
    typealias Param = (inout TheWorld,)
}
 
 
// // Implement Resource for Timer.
// struct Timer: Resource {
//     var value: Int = 0
// }
// 
// func run_timer(_: Timer) { }

extension Sys1: IntoSystem { }

"TODO"

// App.new()
//   .insert_resource(Timer())
//   .add_system(run_timer)
//   .run()
